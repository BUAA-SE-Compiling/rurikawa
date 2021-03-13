use super::exec::BuildResultChannel;
use super::model::*;
use super::utils::convert_code;
use super::{JobFailure, ProcessInfo};
use crate::{prelude::*, sh};
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::UploadToContainerOptions, exec::StartExecResults, models::Mount, Docker};
use drop_bomb::DropBomb;
use futures::stream::StreamExt;
use names::{Generator, Name};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::{collections::HashMap, default::Default};
use tokio::process::Command;
use tokio_util::compat::*;

/// An evaluation environment for commands.
#[async_trait]
pub trait CommandRunner {
    /// Evaluate a command string with the given variables to replace.
    /// The command should be supplied with Unix Shell style.
    async fn run(&self, cmd: &str, variables: &HashMap<String, String>)
        -> PopenResult<ProcessInfo>;
}

/// A *local* command evaluation environment.
/// This is used generally for local testing purposes.
pub struct TokioCommandRunner {}

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(
        &self,
        cmd_str: &str,
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo> {
        let cmd: Vec<String> = sh!(cmd_str);

        let (car, cdr) = cmd.split_first().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Command must contain at least one string",
            )
        })?;

        let mut command = Command::new(car);
        command.args(cdr);

        for (k, v) in variables {
            command.env(k, v);
        }

        let std::process::Output {
            status,
            stdout,
            stderr,
        } = command.output().await?;

        let ret_code = ret_code_from_exit_status(status);
        let ret_code = convert_code(ret_code);

        Ok(ProcessInfo {
            command: cmd_str.to_owned(),
            is_user_command: false,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            ret_code,
        })
    }
}

#[cfg(windows)]
fn ret_code_from_exit_status(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(unix)]
fn ret_code_from_exit_status(status: ExitStatus) -> i32 {
    status
        .code()
        .or_else(|| status.signal().map(|x| -x))
        .unwrap_or(1)
}

/// Command evaluation environment in a Docker container.
///
/// Attention:
/// - Every `DockerCommandRunner` instance includes a `DropBomb`,
///     which prevents `drop`ping without explicitly using `self.kill()`.
/// - When the instance is directly `drop`ped, a runtime panic will occur.
pub struct DockerCommandRunner {
    /// The image to be used.
    image: Image,
    /// A connection to the Docker daemon.
    instance: Docker,
    /// Options while operating the runner.
    options: DockerCommandRunnerOptions,
    /// Intermediate images created by this runner.
    pub intermediate_images: Vec<String>,
    /// A bomb that must be defused. Prevents drops without explicit kills.
    bomb: DropBomb,
}

/// The options while creating a `DockerCommandRunner`.
pub struct DockerCommandRunnerOptions {
    /// Name assigned to the container.
    pub container_name: String,
    /// Memory limit of the container.
    pub mem_limit: Option<usize>,
    /// If the image needs to be pulled/built before run.
    pub build_image: bool,
    /// If the image needs to be removed after run.
    pub remove_image: bool,
    /// If the list of intermediate images created by this runner needs to be recorded.
    pub record_intermediate_images: bool,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<Mount>>,
    /// Data to be copied into container before build, in format of `(source_dir, target_dir)`
    pub copies: Option<Vec<(String, String)>>,
    /// Token to cancel this runner
    pub cancellation_token: CancellationTokenHandle,
}

impl Default for DockerCommandRunnerOptions {
    fn default() -> Self {
        let mut names = Generator::with_naming(Name::Numbered);
        DockerCommandRunnerOptions {
            container_name: format!("rurikawa_{}", names.next().unwrap()),
            mem_limit: None,
            build_image: false,
            remove_image: false,
            record_intermediate_images: false,
            binds: None,
            copies: None,
            cancellation_token: Default::default(),
        }
    }
}

impl DockerCommandRunner {
    /// Try creating a new `DockerCommandRunner` instance.
    ///
    /// This includes:
    /// - Defusing the DropBomb.
    /// - Stopping & removing the container.
    /// - Removing all the intermediate images (only if `self.options.remove_image` is set to `true`).
    // ! WARNING: When implementing this function, THE QUESTION MARK SHALL NEVER BE USED
    // ! as it implies an implicit drop of `self`, which is not tolerated!
    pub async fn try_new(
        instance: Docker,
        image: Image,
        options: DockerCommandRunnerOptions,
        partial_result_channel: Option<BuildResultChannel>,
    ) -> Result<Self> {
        let mut r = DockerCommandRunner {
            image,
            instance,
            options,
            intermediate_images: vec![],
            bomb: DropBomb::new(
                "DockerCommandRunner must be explicitly killed to prevent stranding contrainers",
            ),
        };

        /// The equivalent of Rust's `try` macro, with the only difference that
        /// right before early returning errors, `DockerCommandRunner` is killed.
        // TODO: When `AsyncDrop` is stabled, use RAII + kill() instead of this workaround.
        macro_rules! try_or_kill {
            ($res:expr $(,)?) => {
                match $res {
                    Ok(val) => val,
                    Err(err) => {
                        r.kill().await;
                        return Err(err.into());
                    }
                }
            };
        }

        let cancel = r.options.cancellation_token.clone();

        log::info!("container {}: started building", r.options.container_name);

        // Build the image.
        if r.options.build_image {
            try_or_kill!(
                r.image
                    .build(r.instance.clone(), partial_result_channel, cancel.clone(),)
                    .await
            )
        };

        let mut image_name = r.image.tag();
        if r.options.record_intermediate_images {
            r.intermediate_images.push(image_name.clone());
        }

        // Copy data into the container.
        if let Some(copies) = &r.options.copies {
            let after_copy_image_name = format!("{}_copied", image_name);

            let container_name = format!(
                "{}-add-data-{}",
                r.options.container_name,
                FlowSnake::generate()
            );
            log::info!(
                "Preparing to copy files into {}; to create container {}",
                image_name,
                container_name
            );

            let create_res = r
                .instance
                .create_container(
                    Some(bollard::container::CreateContainerOptions {
                        name: container_name.clone(),
                    }),
                    bollard::container::Config {
                        image: Some(image_name.clone()),
                        tty: Some(true),
                        open_stdin: Some(true),
                        attach_stdin: Some(true),
                        entrypoint: Some(vec!["sh".into()]),
                        ..Default::default()
                    },
                )
                .with_cancel(cancel)
                .await;

            // Ensure every early return comes with an explicit kill.
            if create_res.is_none() {
                // TODO: Cleanup
                r.kill().await;
                return Err(JobFailure::Cancelled.into());
            } else if let Err(e) = create_res.unwrap() {
                r.kill().await;
                return Err(JobFailure::internal_err_from(format!(
                    "Failed to create container `{}`: {}",
                    &container_name, e
                ))
                .into());
            }

            // Start the container.
            try_or_kill!(
                r.instance
                    .start_container::<String>(&container_name, None)
                    .await,
            );

            log::info!("created container {}", container_name);

            // Copy files.
            for (from_path, to_path) in copies {
                log::info!("Copying {} to {} in {}", from_path, to_path, image_name);

                let exec = try_or_kill!(
                    r.instance
                        .create_exec(
                            &container_name,
                            bollard::exec::CreateExecOptions {
                                cmd: Some(vec!["mkdir", "-p", to_path]),
                                attach_stdout: Some(true),
                                attach_stderr: Some(true),
                                ..Default::default()
                            },
                        )
                        .await
                );

                let mut exec_res = r
                    .instance
                    .start_exec(
                        &exec.id,
                        Some(bollard::exec::StartExecOptions { detach: false }),
                    )
                    .map(|x| x.map(|_| ()))
                    .collect::<Vec<_>>()
                    .await;

                try_or_kill!(exec_res
                    .drain(..)
                    .collect::<Result<Vec<_>, bollard::errors::Error>>());

                let from_path = from_path.clone();
                let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
                let read_codec = tokio_util::codec::BytesCodec::new();
                let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);
                let task = async move {
                    let mut tar = async_tar::Builder::new(futures::io::BufWriter::new(
                        pipe_recv.compat_write(),
                    ));
                    match tar.append_dir_all(".", from_path).await {
                        Ok(_) => tar.finish().await,
                        e @ Err(_) => e,
                    }
                };
                let task = tokio::spawn(task);
                try_or_kill!(
                    r.instance
                        .upload_to_container(
                            &container_name,
                            Some(UploadToContainerOptions {
                                path: to_path.clone(),
                                ..Default::default()
                            }),
                            hyper::Body::wrap_stream(frame.map(|x| x)),
                        )
                        .await
                );
                try_or_kill!(try_or_kill!(task.await));
            }

            try_or_kill!(
                r.instance
                    .commit_container(
                        bollard::image::CommitContainerOptions {
                            container: container_name.clone(),
                            repo: after_copy_image_name.clone(),
                            ..Default::default()
                        },
                        bollard::container::Config::<String>::default(),
                    )
                    .await
            );

            if r.options.record_intermediate_images {
                r.intermediate_images.push(after_copy_image_name.clone());
            }
            image_name = after_copy_image_name;

            try_or_kill!(r.instance.stop_container(&container_name, None).await);
            r.instance
                .wait_container::<String>(&container_name, None)
                .collect::<Vec<_>>()
                .await;
            try_or_kill!(r.instance.remove_container(&container_name, None).await);
        }

        log::trace!("container {}: creating", r.options.container_name);

        // Create a container
        try_or_kill!(r
            .instance
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: r.options.container_name.clone(),
                }),
                bollard::container::Config {
                    image: Some(image_name),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    host_config: Some(bollard::service::HostConfig {
                        mounts: r.options.binds.clone(),
                        ..Default::default()
                    }),
                    entrypoint: Some(vec!["sh".into()]),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to create container `{}`: {}",
                    &r.options.container_name, e
                ))
            }));

        let container_name = &r.options.container_name;

        // Set memory limit
        try_or_kill!(r
            .instance
            .update_container(
                container_name,
                bollard::container::UpdateContainerOptions::<String> {
                    memory: r.options.mem_limit.map(|n| n as i64),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to update container `{}`: {}",
                    container_name, e
                ))
            }));

        log::trace!("container {}: starting", r.options.container_name);
        // Start the container
        try_or_kill!(r
            .instance
            .start_container::<String>(container_name, None)
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to start container `{}`: {}",
                    container_name, e
                ))
            }),);

        log::trace!("container {}: launched", r.options.container_name);
        Ok(r)
    }

    /// Kill the `DockerCommandRunner` instance.
    ///
    /// This includes:
    /// - Defusing the DropBomb.
    /// - Stopping & removing the container.
    /// - Removing all the intermediate images (only if `self.options.remove_image` is set to `true`).
    // ! WARNING: When implementing this function, we should explicitly drop the returned values because we have no way to fail.
    pub async fn kill(mut self) {
        // Defuse the bomb.
        self.bomb.defuse();

        let container_name = &self.options.container_name;

        let _res = self
            .instance
            .stop_container(
                container_name,
                Some(bollard::container::StopContainerOptions { t: 15 }),
            )
            .await;

        let _res = self
            .instance
            .wait_container::<String>(container_name, None)
            .for_each(|_| async {})
            .await;

        let _res = self
            .instance
            .remove_container(
                container_name,
                None::<bollard::container::RemoveContainerOptions>,
            )
            .await;

        // Remove the image.
        if self.options.remove_image {
            for image in &self.intermediate_images {
                let _res = self
                    .instance
                    .remove_image(
                        image,
                        Some(bollard::image::RemoveImageOptions {
                            ..Default::default()
                        }),
                        None,
                    )
                    .await;
            }
        }
    }
}

// 100kB
// TODO: user-configurable output size
static MAX_CONSOLE_FILE_SIZE: usize = 100 * 1024;

#[async_trait]
impl CommandRunner for DockerCommandRunner {
    async fn run(
        &self,
        cmd: &str,
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo> {
        let container_name = &self.options.container_name;

        // Create a Docker Exec
        let env = variables
            .iter()
            .map(|(k, v)| format!("{}={}", k.trim_start_matches('$'), v))
            .collect::<Vec<_>>();

        let message = self
            .instance
            .create_exec(
                container_name,
                bollard::exec::CreateExecOptions {
                    cmd: Some(vec!["sh", "-c", &cmd]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    env: Some(env.iter().map(|x| x.as_str()).collect()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create Docker Exec: {}", e),
                )
            })?;

        // Start the Docker Exec
        let mut start_res = self.instance.start_exec(
            &message.id,
            Some(bollard::exec::StartExecOptions { detach: false }),
        );

        let mut stdout = String::new();
        let mut stderr = String::new();

        while let Some(msg) = start_res.next().await {
            match msg {
                Ok(r) => match r {
                    StartExecResults::Attached { log } => match log {
                        bollard::container::LogOutput::StdOut { message } => {
                            let msg = String::from_utf8_lossy(&message);
                            stdout.push_str(&msg);
                            if (stdout.len() >= MAX_CONSOLE_FILE_SIZE) {
                                stdout.push_str("\n--- ERROR: Max output length exceeded");
                                break;
                            }
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            let msg = String::from_utf8_lossy(&message);
                            stderr.push_str(&msg);
                            if (stderr.len() >= MAX_CONSOLE_FILE_SIZE) {
                                stderr.push_str("\n--- ERROR: Max output length exceeded");
                                break;
                            }
                        }
                        _ => {}
                    },
                    StartExecResults::Detached => {}
                },
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        }

        drop(start_res);

        // Use inspect_exec to get exit code.
        let inspect_res = self.instance.inspect_exec(&message.id).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to inspect Docker Exec: {:?}", e),
            )
        })?;
        let ret_code = inspect_res
            .exit_code
            .map(|x| convert_code(x as i32))
            .unwrap_or(-1);

        Ok(ProcessInfo {
            command: cmd.into(),
            is_user_command: false,
            stdout,
            stderr,
            ret_code,
        })
    }
}
