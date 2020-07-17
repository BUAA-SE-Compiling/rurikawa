use super::ProcessInfo;
use crate::prelude::*;
use async_trait::async_trait;
use futures::stream::Stream;
use shiplift::tty::StreamType;
use shiplift::{Container, ExecContainerOptions};
use tokio::process::Command;

#[async_trait]
pub trait CommandRunner {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<std::process::Output>;
}

pub struct TokioCommandRunner {}

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<std::process::Output> {
        let mut cmd = cmd.iter();
        let mut command = Command::new(cmd.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Command must contain at least one string",
            )
        })?);
        command.args(cmd);
        command.output().await
    }
}

pub struct DockerCommandRunner<'a, 'b> {
    container: Container<'a, 'b>,
}

#[async_trait]
impl<'a, 'b> CommandRunner for DockerCommandRunner<'a, 'b> {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<std::process::Output> {
        let options = ExecContainerOptions::builder()
            .cmd(cmd.iter().map(|x| x as &str).collect())
            .attach_stdout(true)
            .attach_stderr(true)
            .build();
        let running = self.container.exec(&options);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let f = running.for_each(|mut chunk| {
            match chunk.stream_type {
                StreamType::StdOut => stdout.append(&mut chunk.data),
                StreamType::StdErr => stderr.append(&mut chunk.data),
                _ => {}
            }
            futures::finished(())
        });
        // TODO: the following line cannot compile
        // f.await;
        todo!("Finish docker command runner")
    }
}
