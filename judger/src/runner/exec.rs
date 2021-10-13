use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, UploadToContainerOptions},
    exec::{CreateExecOptions, StartExecOptions},
    models::{ContainerConfig, Mount},
    Docker,
};
use derive_builder::Builder;
use ignore::gitignore::Gitignore;
use tokio_stream::StreamExt;

use crate::{
    prelude::CancellationTokenHandle,
    runner::util::is_recoverable_error,
    tester::{model::Bind, ProcessInfo},
    util::tar::pack_as_tar,
};

use super::model::CommandRunner;

#[derive(Debug, Builder)]
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
    name: String,
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
            name: res.id,
            tag: cfg.tag_name,
            state: ContainerState::Stopped,
        })
    }

    pub async fn copy_local_files(&self, file_path: &Path, into_path: &str) -> anyhow::Result<()> {
        let (tar, join) = pack_as_tar(file_path, Gitignore::empty())?;
        self.docker
            .upload_to_container(
                &self.name,
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

    pub async fn exec(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
    ) -> anyhow::Result<ProcessInfo> {
        let exec = self
            .docker
            .create_exec(
                &self.name,
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

        let exec = self
            .docker
            .start_exec(&exec.id, Some(StartExecOptions { detach: false }))
            .await?;

        let mut output = match exec {
            bollard::exec::StartExecResults::Attached { output, input: _ } => output,
            bollard::exec::StartExecResults::Detached => unreachable!("All exec are attached"),
        };

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
                bollard::container::LogOutput::StdErr { message } => todo!(),
                bollard::container::LogOutput::StdOut { message } => todo!(),
                bollard::container::LogOutput::StdIn { message } => todo!(),
                bollard::container::LogOutput::Console { message } => todo!(),
            }
        }

        todo!()
    }
}

#[async_trait]
impl CommandRunner for Container {
    async fn run(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
    ) -> anyhow::Result<ProcessInfo> {
        self.exec(command, env).await
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        format!("container {}", self.name).into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerState {
    Empty,
    Stopped,
    Running,
}

#[derive(Debug, Builder, Default)]
pub struct ExecOptions {}
