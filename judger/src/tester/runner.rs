use super::ProcessInfo;
use crate::prelude::*;
use async_trait::async_trait;
use bollard::Docker;
use futures::stream::StreamExt;
use std::default::Default;
use std::os::unix::process::ExitStatusExt;
use tokio::process::Command;

#[async_trait]
pub trait CommandRunner {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<ProcessInfo>;
}

pub struct TokioCommandRunner {}

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<ProcessInfo> {
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
        let ret_code = match (status.code(), status.signal()) {
            (Some(x), _) => x,
            (None, Some(x)) => -x,
            _ => unreachable!(),
        };
        Ok(ProcessInfo {
            command: cmd_str,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            ret_code,
        })
    }
}

pub struct DockerCommandRunner {
    instance: Docker,
    // TODO: What is the container name?
    container_name: String,
}

impl<'d> DockerCommandRunner {
    pub async fn new(instance: Docker, container_name: &str, image_name: &str) -> Self {
        let res = DockerCommandRunner {
            instance,
            container_name: container_name.to_owned(),
        };
        // TODO: If the image is not yet pulled, pull it before continuing.
        // Create a container
        res.instance
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: container_name,
                }),
                bollard::container::Config {
                    image: Some(image_name),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_else(|e| panic!("Failed to create Docker instance: {}", e));
        res.instance
            .start_container(
                container_name,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await
            .unwrap_or_else(|_| panic!("Failed to start Docker container {}", container_name));
        res
    }
}

#[async_trait]
impl CommandRunner for DockerCommandRunner {
    async fn run(&mut self, cmd: &[String]) -> PopenResult<ProcessInfo> {
        let cmd_str = format!("{:?}", cmd.to_vec());

        // Create a Docker Exec
        let message = self
            .instance
            .create_exec(
                &self.container_name,
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
        let start_res = self.instance.start_exec(
            &message.id,
            Some(bollard::exec::StartExecOptions { detach: false }),
        );

        let messages: Vec<MessageKind> = start_res
            .filter_map(|mres| async {
                match mres {
                    Ok(bollard::exec::StartExecResults::Attached { log }) => match log {
                        bollard::container::LogOutput::StdOut { message } => {
                            Some(MessageKind::StdOut(message))
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            Some(MessageKind::StdErr(message))
                        }
                        _ => None,
                    },
                    _ => None,
                }
            })
            .collect()
            .await;

        let (stdout, stderr): (Vec<&MessageKind>, Vec<&MessageKind>) = messages
            .iter()
            .partition(|&i| matches!(i, &MessageKind::StdOut(_)));

        let mut stdout = stdout
            .iter()
            .map(|&i| i.unwrap())
            .collect::<Vec<String>>()
            .join("");
        stdout.push_str("\n");
        let mut stderr = stderr
            .iter()
            .map(|&i| i.unwrap())
            .collect::<Vec<String>>()
            .join("");
        stderr.push_str("\n");

        // Use inspect_exec to get exit code.
        let inspect_res = self.instance.inspect_exec(&message.id).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to inspect Docker Exec: {:?}", e),
            )
        })?;

        let ret_code = inspect_res.exit_code.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to fetch Docker Exec exit code",
            )
        })?;

        Ok(ProcessInfo {
            command: cmd_str,
            stdout,
            stderr,
            ret_code: ret_code as i32,
        })
    }
}

/// Helper enum for DockerCommandRunner
enum MessageKind {
    StdOut(String),
    StdErr(String),
}

impl MessageKind {
    fn unwrap(&self) -> String {
        match self {
            MessageKind::StdOut(s) => s.to_owned(),
            MessageKind::StdErr(s) => s.to_owned(),
        }
    }
}
