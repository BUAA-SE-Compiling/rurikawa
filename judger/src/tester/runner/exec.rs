use std::sync::Arc;

use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions},
    exec::CreateExecOptions,
    models::ContainerConfig,
    Docker,
};
use derive_builder::Builder;

use crate::tester::ProcessInfo;

use super::model::CommandRunner;
use super::model::ExecStep;

#[derive(Debug)]
pub struct Container {
    docker: Docker,
    name: String,
    state: ContainerState,
}

impl Container {
    pub async fn create(
        docker: Docker,
        name: String,
        image: String,
        cfg: bollard::container::Config<String>,
    ) -> Result<Self, bollard::errors::Error> {
        let res = docker
            .create_container(
                Some(CreateContainerOptions { name }),
                Config {
                    image: Some(image),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    // set docker user
                    // user: r.options.cfg.docker_user.clone(),
                    host_config: Some(bollard::service::HostConfig {
                        // mounts: r.options.binds.clone(),
                        // // set memory limits
                        // memory_swap: r.options.mem_limit.map(|n| n as i64),
                        // // set cpu limits
                        // nano_cpus: r.options.cfg.run_cpu_share.map(|x| (x * 1e9) as i64),
                        ..Default::default()
                    }),
                    entrypoint: Some(vec!["sh".into()]),
                    // Set network availability
                    // network_disabled: Some(!r.options.network_options.enable_running),
                    ..Default::default()
                },
            )
            .await?;
        Ok(Container {
            docker,
            name: res.id,
            state: ContainerState::Stopped,
        })
    }

    pub async fn exec(&self, command: &str) -> ProcessInfo {
        let exec = self
            .docker
            .create_exec(
                &self.name,
                CreateExecOptions {
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    env: Some(vec!["CI=true"]),
                    cmd: Some(vec![command]),
                    ..Default::default()
                },
            )
            .await;

        todo!()
    }
}

#[async_trait]
impl CommandRunner for Container {
    async fn run(&self, command: String) -> ProcessInfo {
        self.exec(&command).await
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
