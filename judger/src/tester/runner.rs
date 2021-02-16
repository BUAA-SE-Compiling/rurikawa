use super::exec::BuildResultChannel;
use super::model::*;
use super::utils::convert_code;
use super::{JobFailure, ProcessInfo};
use crate::prelude::*;
use anyhow::Result;
use async_trait::async_trait;
use bollard::{container::UploadToContainerOptions, exec::StartExecResults, models::Mount, Docker};
use futures::stream::StreamExt;
use names::{Generator, Name};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::{default::Default, time::Duration};
use tokio::{io::BufWriter, process::Command};
use tokio_util::compat::*;

/// An evaluation environment for commands.
#[async_trait]
pub trait CommandRunner {
    /// Evaluate a command.
    async fn run(&self, cmd: &[String]) -> PopenResult<ProcessInfo>;
}

/// A *local* command evaluation environment.
pub struct TokioCommandRunner {}

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(&self, cmd: &[String]) -> PopenResult<ProcessInfo> {
        let cmd_str = format!("{:?}", cmd.to_vec());
        let mut cmd_iter = cmd.iter();
        let mut command = Command::new(cmd_iter.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Command must contain at least one string",
            )
        })?);
        command.args(cmd_iter);
        let std::process::Output {
            status,
            stdout,
            stderr,
        } = command.output().await?;
        let ret_code = ret_code_from_exit_status(status);
        let ret_code = convert_code(ret_code);
        Ok(ProcessInfo {
            command: cmd_str,
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
    match (status.code(), status.signal()) {
        (Some(x), _) => x,
        (None, Some(x)) => -x,
        _ => unreachable!(),
    }
}

/// Command evaluation environment in a Docker container.
pub struct DockerCommandRunner {
    /// The image to be used.
    image: Image,
    /// A connection to the Docker daemon.
    instance: Docker,
    /// Options while operating the runner.
    options: DockerCommandRunnerOptions,
    /// Intermediate images created by this runner
    pub intermediate_images: Vec<String>,
}

pub struct DockerCommandRunnerOptions {
    /// Name assigned to the container.
    pub container_name: String,
    /// Memory limit of the container.
    pub mem_limit: Option<usize>,
    /// If the image needs to be pulled/built before run.
    pub build_image: bool,
    /// If the image needs to be removed after run.
    pub remove_image: bool,
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
            binds: None,
            copies: None,
            cancellation_token: Default::default(),
        }
    }
}

impl DockerCommandRunner {
    pub async fn try_new(
        instance: Docker,
        image: Image,
        options: DockerCommandRunnerOptions,
        partial_result_channel: Option<BuildResultChannel>,
    ) -> Result<Self> {
        let mut res = DockerCommandRunner {
            image,
            instance,
            options,
            intermediate_images: vec![],
        };
        let cancel = res.options.cancellation_token.clone();

        log::info!("container {}: started building", res.options.container_name);

        // Build the image
        if res.options.build_image {
            res.image
                .build(res.instance.clone(), partial_result_channel, cancel.clone())
                .await?
        };
        let mut image_name = res.image.tag();

        res.intermediate_images.push(image_name.clone());

        // Copy data into the container
        if let Some(copies) = &res.options.copies {
            let after_copy_image_name = format!("{}_copied", image_name);

            let container_name = format!(
                "{}-add-data-{}",
                res.options.container_name,
                FlowSnake::generate()
            );
            log::info!(
                "Preparing to copy files into {}; to create container {}",
                image_name,
                container_name
            );

            let create_instance = res
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
                .with_cancel(cancel.clone())
                .await;
            match create_instance {
                Some(r) => r.map_err(|e| {
                    JobFailure::internal_err_from(format!(
                        "Failed to create container `{}`: {}",
                        &container_name, e
                    ))
                })?,
                None => {
                    // TODO: Cleanup
                    res.kill().await;
                    return Err(JobFailure::Cancelled.into());
                }
            };

            res.instance
                .start_container::<String>(&container_name, None)
                .await?;

            log::info!("created container {}", container_name);

            for (from_path, to_path) in copies {
                log::info!("Copying {} to {} in {}", from_path, to_path, image_name);

                let exec = res
                    .instance
                    .create_exec(
                        &container_name,
                        bollard::exec::CreateExecOptions {
                            cmd: Some(vec!["mkdir", "-p", to_path]),
                            attach_stdout: Some(true),
                            attach_stderr: Some(true),
                            ..Default::default()
                        },
                    )
                    .await?;

                let mut exec_res = res
                    .instance
                    .start_exec(
                        &exec.id,
                        Some(bollard::exec::StartExecOptions { detach: false }),
                    )
                    .map(|x| x.map(|_| ()))
                    .collect::<Vec<_>>()
                    .await;

                exec_res
                    .drain(..)
                    .collect::<Result<Vec<_>, bollard::errors::Error>>()?;

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
                res.instance
                    .upload_to_container(
                        &container_name,
                        Some(UploadToContainerOptions {
                            path: to_path.clone(),
                            ..Default::default()
                        }),
                        hyper::Body::wrap_stream(frame.map(|x| x.map(|x| x))),
                    )
                    .await?;
                task.await??;
            }

            res.instance
                .commit_container(
                    bollard::image::CommitContainerOptions {
                        container: container_name.clone(),
                        repo: after_copy_image_name.clone(),
                        ..Default::default()
                    },
                    bollard::container::Config::<String>::default(),
                )
                .await?;

            res.intermediate_images.push(after_copy_image_name.clone());
            image_name = after_copy_image_name;

            res.instance.stop_container(&container_name, None).await?;
            res.instance
                .wait_container::<String>(&container_name, None)
                .collect::<Vec<_>>()
                .await;
            res.instance.remove_container(&container_name, None).await?;
        }

        log::trace!("container {}: creating", res.options.container_name);

        // Create a container
        res.instance
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: res.options.container_name.clone(),
                }),
                bollard::container::Config {
                    image: Some(image_name),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    host_config: Some(bollard::service::HostConfig {
                        mounts: res.options.binds.clone(),
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
                    &res.options.container_name, e
                ))
            })?;

        let container_name = &res.options.container_name;

        // Set memory limit
        res.instance
            .update_container(
                container_name,
                bollard::container::UpdateContainerOptions::<String> {
                    memory: res.options.mem_limit.map(|n| n as i64),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to update container `{}`: {}",
                    container_name, e
                ))
            })?;

        log::trace!("container {}: starting", res.options.container_name);
        // Start the container
        res.instance
            .start_container(
                container_name,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to start container `{}`: {}",
                    container_name, e
                ))
            })?;

        log::trace!("container {}: finished", res.options.container_name);
        Ok(res)
    }

    pub async fn kill(self) {
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
            .wait_container(
                container_name,
                None::<bollard::container::WaitContainerOptions<String>>,
            )
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
    async fn run(&self, cmd: &[String]) -> PopenResult<ProcessInfo> {
        let cmd_str = format!("{:?}", cmd.to_vec());
        let container_name = &self.options.container_name;

        // Create a Docker Exec
        let message = self
            .instance
            .create_exec(
                container_name,
                bollard::exec::CreateExecOptions {
                    cmd: Some(cmd.to_vec()),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
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
            command: cmd_str,
            stdout,
            stderr,
            ret_code,
        })
    }
}
