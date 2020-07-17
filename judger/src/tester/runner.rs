use super::ProcessInfo;
use crate::prelude::*;
use async_trait::async_trait;
use tokio::process::Command;

#[async_trait]
pub trait CommandRunner {
    async fn run(&mut self, cmd: &mut Command) -> PopenResult<std::process::Output>;
}

pub struct TokioCommandRunner {}

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(&mut self, cmd: &mut Command) -> PopenResult<std::process::Output> {
        cmd.output().await
    }
}

pub struct DockerCommandRunner {}
