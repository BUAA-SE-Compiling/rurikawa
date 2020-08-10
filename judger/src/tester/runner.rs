use super::exec::Image;
use super::utils::convert_code;
use super::{JobFailure, ProcessInfo};
use crate::prelude::*;
use anyhow::Result;
use async_trait::async_trait;
use bollard::Docker;
use futures::stream::StreamExt;
use names::{Generator, Name};
use std::default::Default;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::{collections::HashMap, process::ExitStatus};
use tokio::process::Command;

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
    /// A connection to the Docker daemon.
    instance: Docker,
    /// Name of the container in which the command is evaluated.
    container_name: String,
}

pub struct DockerCommandRunnerOptions {
    pub container_name: String,
    pub mem_limit: Option<usize>,
    /// If the image needs to be built before run.
    pub build_image: bool,
    pub volumes: HashMap<String, String>,
}

impl Default for DockerCommandRunnerOptions {
    fn default() -> Self {
        let mut names = Generator::with_naming(Name::Numbered);
        DockerCommandRunnerOptions {
            container_name: names.next().unwrap(),
            mem_limit: None,
            build_image: false,
            volumes: HashMap::new(),
        }
    }
}

impl DockerCommandRunner {
    pub async fn try_new(
        instance: Docker,
        image: Image,
        options: DockerCommandRunnerOptions,
    ) -> Result<Self> {
        let DockerCommandRunnerOptions {
            container_name,
            mem_limit,
            build_image,
            volumes,
        } = options;
        let res = DockerCommandRunner {
            instance,
            container_name,
        };

        /*
        // Pull the image
        res.instance
            .create_image(
                Some(bollard::image::CreateImageOptions {
                    from_image: image_name,
                    ..Default::default()
                }),
                None,
                None,
            )
            .map(|mr| {
                mr.unwrap_or_else(|e| panic!("Failed to pull Docker image `{}`: {}", image_name, e))
            })
            .collect::<Vec<_>>()
            .await;
        */

        // Build the image
        if build_image {
            image.build(res.instance.clone()).await?
        };
        let image_name = image.tag();

        // Create a container
        res.instance
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: res.container_name.clone(),
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
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to create container `{}`: {}",
                    &res.container_name, e
                ))
            })?;

        // Set memory limit
        res.instance
            .update_container(
                &res.container_name,
                bollard::container::UpdateContainerOptions::<String> {
                    memory: mem_limit.map(|n| n as i64),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to update container `{}`: {}",
                    &res.container_name, e
                ))
            })?;

        // Start the container
        res.instance
            .start_container(
                &res.container_name,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await
            .map_err(|e| {
                JobFailure::internal_err_from(format!(
                    "Failed to start container `{}`: {}",
                    &res.container_name, e
                ))
            })?;

        Ok(res)
    }

    pub async fn kill(self) {
        self.instance
            .kill_container(
                &self.container_name,
                None::<bollard::container::KillContainerOptions<String>>,
            )
            .await
            .unwrap();

        self.instance
            .wait_container(
                &self.container_name,
                None::<bollard::container::WaitContainerOptions<String>>,
            )
            .collect::<Vec<_>>()
            .await;

        self.instance
            .remove_container(
                &self.container_name,
                None::<bollard::container::RemoveContainerOptions>,
            )
            .await
            .unwrap();
    }
}

#[async_trait]
impl CommandRunner for DockerCommandRunner {
    async fn run(&self, cmd: &[String]) -> PopenResult<ProcessInfo> {
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
                            let message = String::from_utf8((*message).to_vec()).unwrap();
                            Some(MessageKind::StdOut(message))
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            let message = String::from_utf8((*message).to_vec()).unwrap();
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

        let stdout = stdout
            .iter()
            .map(|&i| i.unwrap())
            .collect::<Vec<String>>()
            .join("");
        let stderr = stderr
            .iter()
            .map(|&i| i.unwrap())
            .collect::<Vec<String>>()
            .join("");

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
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to fetch Docker Exec exit code",
                )
            })?;

        Ok(ProcessInfo {
            command: cmd_str,
            stdout,
            stderr,
            ret_code,
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
