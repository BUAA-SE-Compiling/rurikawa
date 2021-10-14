use std::{fmt::Write, path::Path};

use async_trait::async_trait;
use bollard::{
    container::{Config, UploadToContainerOptions},
    exec::{CreateExecOptions, StartExecOptions},
    models::Mount,
    Docker,
};
use bytes::BytesMut;
use derive_builder::Builder;
use ignore::gitignore::Gitignore;
use tokio_stream::StreamExt;

use crate::{
    prelude::CancellationTokenHandle, runner::model::ProcessOutput,
    runner::util::is_recoverable_error, util::tar::pack_as_tar,
};

use super::model::{CommandRunOptions, CommandRunner};

#[derive(Debug, Builder)]
#[builder(setter(into, strip_option))]
pub struct CreateContainerConfig {
    /// Mounting local folders into containers
    #[builder(default)]
    mounts: Vec<Mount>,

    /// A tag for this container. Not used in any docker commands, purely for labelling & debugging use.
    #[builder(default)]
    tag_name: Option<String>,

    /// The user to be used when running docker commands
    #[builder(default)]
    docker_user: Option<String>,

    /// A handle to cancel this run
    #[builder(default)]
    cancellation: CancellationTokenHandle,

    /// The memory limit of this container
    #[builder(default)]
    mem_limit: Option<i64>,

    /// The CPU fraction allowed to use
    #[builder(default)]
    cpu_quota: Option<f64>,

    /// Whether network is allowed in this container
    #[builder(default = "false")]
    network_enabled: bool,
}

#[derive(Debug)]
struct ContainerId(String);

#[derive(Debug)]
pub struct Container {
    docker: Docker,
    id: String,
    tag: Option<String>,
    state: ContainerState,
}

impl Container {
    pub async fn create(
        docker: Docker,
        image: String,
        cfg: CreateContainerConfig,
    ) -> Result<Self, bollard::errors::Error> {
        let res = docker
            .create_container::<String, _>(
                None,
                Config {
                    image: Some(image),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    // set docker user
                    user: cfg.docker_user,
                    host_config: Some(bollard::service::HostConfig {
                        mounts: Some(cfg.mounts),
                        // set memory limits
                        memory_swap: cfg.mem_limit,
                        // set cpu limits
                        nano_cpus: cfg.cpu_quota.map(|x| (x * 1e9) as i64),
                        ..Default::default()
                    }),
                    entrypoint: Some(vec!["sh".into()]),
                    // Set network availability
                    network_disabled: Some(!cfg.network_enabled),
                    ..Default::default()
                },
            )
            .await?;
        Ok(Container {
            docker,
            id: res.id,
            tag: cfg.tag_name,
            state: ContainerState::Stopped,
        })
    }

    pub async fn copy_local_files(&self, file_path: &Path, into_path: &str) -> anyhow::Result<()> {
        let (tar, join) = pack_as_tar(file_path, Gitignore::empty())?;
        self.docker
            .upload_to_container(
                &self.id,
                Some(UploadToContainerOptions {
                    path: into_path,
                    no_overwrite_dir_non_dir: "false",
                }),
                hyper::Body::wrap_stream(tar),
            )
            .await?;
        join.await??;
        Ok(())
    }

    /// Execute a certain `command` in a certain `env`ironment
    pub async fn exec(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
        opt: &CommandRunOptions,
    ) -> anyhow::Result<ProcessOutput> {
        let exec = self
            .docker
            .create_exec(
                &self.id,
                CreateExecOptions {
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    env: Some(env.map(|(k, v)| format!("{}={}", k, v)).collect()),
                    cmd: Some(vec![command.into()]),
                    ..Default::default()
                },
            )
            .await?;

        let exec_id = &exec.id;
        let exec = self
            .docker
            .start_exec(exec_id, Some(StartExecOptions { detach: false }))
            .await?;

        let mut output = match exec {
            bollard::exec::StartExecResults::Attached { output, input: _ } => output,
            bollard::exec::StartExecResults::Detached => unreachable!("All exec are attached"),
        };

        let mut stdout = SizeConstraintBytesMut::new(opt.stdout_size_limit);
        let mut stderr = SizeConstraintBytesMut::new(opt.stderr_size_limit);

        while let Some(v) = output.next().await {
            let out = match v {
                Ok(out) => out,
                Err(e) => {
                    if is_recoverable_error(&e) {
                        continue;
                    } else {
                        return Err(e.into());
                    }
                }
            };

            match out {
                bollard::container::LogOutput::StdErr { message } => stderr.append(&message),
                bollard::container::LogOutput::StdOut { message } => stdout.append(&message),
                bollard::container::LogOutput::StdIn { .. } => {}
                bollard::container::LogOutput::Console { .. } => {}
            }
        }

        let results = self.docker.inspect_exec(exec_id).await?;
        let ret_code = results.exit_code;

        Ok(ProcessOutput {
            ret_code: ret_code.map_or(-1, |x| x as i32),
            command: command.to_string(),
            stdout: stdout.into_string(),
            stderr: stderr.into_string(),

            runned_inside: self.name().into(),
        })
    }
}

#[async_trait]
impl CommandRunner for Container {
    async fn run(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
        opt: &CommandRunOptions,
    ) -> anyhow::Result<ProcessOutput> {
        self.exec(command, env, opt).await
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        if let Some(tag) = &self.tag {
            format!("Container {} ({})", tag, self.id).into()
        } else {
            format!("Container {}", self.id).into()
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerState {
    Empty,
    Stopped,
    Running,
}

struct SizeConstraintBytesMut {
    size_limit: usize,
    bytes: BytesMut,
}

impl SizeConstraintBytesMut {
    pub fn new(size_limit: usize) -> Self {
        SizeConstraintBytesMut {
            size_limit,
            bytes: BytesMut::new(),
        }
    }

    pub fn append(&mut self, bytes: &[u8]) {
        if self.bytes.len() > self.size_limit {
            // do nothing
        } else if self.bytes.len() + bytes.len() > self.size_limit {
            let cut_at = self.size_limit - self.bytes.len();
            self.bytes.extend_from_slice(&bytes[0..cut_at]);
        } else {
            self.bytes.extend_from_slice(bytes);
        }
    }

    pub fn is_oversized(&self) -> bool {
        self.bytes.len() >= self.size_limit
    }

    pub fn into_string(self) -> String {
        let oversized = self.is_oversized();
        let mut s = String::from_utf8_lossy(&self.bytes).into_owned();
        if oversized {
            writeln!(s).unwrap();
            writeln!(
                s,
                "--- output buffer capped out at {} bytes ---",
                self.size_limit
            )
            .unwrap();
        }
        s
    }
}
