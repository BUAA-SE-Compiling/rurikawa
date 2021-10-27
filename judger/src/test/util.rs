#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;

use bytes::{Bytes, BytesMut};

use tokio::task::JoinHandle;
use tokio_stream::Stream;
use tokio_tar::Header;

use crate::runner::model::{CommandRunOptions, ExitStatus, ProcessOutput};
use crate::runner::CommandRunner;

/// The root directory of this project, where `Cargo.toml` lives in.
pub fn project_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn tar_with_files(
    files: impl Iterator<Item = (String, Bytes)> + Send + 'static,
) -> (
    impl Stream<Item = Result<BytesMut, std::io::Error>> + 'static,
    JoinHandle<()>,
) {
    let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
    let read_codec = tokio_util::codec::BytesCodec::new();
    let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);

    let archiving = tokio::spawn(async move {
        let mut tar = tokio_tar::Builder::new(pipe_recv);
        for (name, file) in files {
            let mut header = Header::new_gnu();
            header.set_path(name).unwrap();
            header.set_mode(0o777);
            tar.append(&header, &*file)
                .await
                .expect("Failed to append file");
        }
        tar.finish().await.expect("Failed to finish tar");
    });

    (frame, archiving)
}

pub struct MockRunner {
    input_output: HashMap<String, ProcessOutput>,
}

impl MockRunner {
    pub fn new() -> Self {
        MockRunner {
            input_output: Default::default(),
        }
    }

    pub fn insert(&mut self, command: String, output: ProcessOutput) {
        self.input_output.insert(command, output);
    }

    pub fn when(&mut self, command: impl Into<String>) -> MockRunnerCommandModifier {
        let command = command.into();
        MockRunnerCommandModifier {
            command: command.clone(),
            runner: self,
            output: ProcessOutput {
                command,
                ..Default::default()
            },
        }
    }
}

#[must_use]
pub struct MockRunnerCommandModifier<'a> {
    command: String,
    output: ProcessOutput,
    runner: &'a mut MockRunner,
}

impl<'a> MockRunnerCommandModifier<'a> {
    pub fn finish(self) {
        self.runner.insert(self.command, self.output);
    }

    pub fn stdout(mut self, stdout: impl Into<String>) -> Self {
        self.output.stdout = stdout.into();
        self
    }

    pub fn stderr(mut self, stderr: impl Into<String>) -> Self {
        self.output.stderr = stderr.into();
        self
    }

    pub fn returns(mut self, code: impl Into<ExitStatus>) -> Self {
        self.output.ret_code = code.into();
        self
    }
}

#[async_trait::async_trait]
impl CommandRunner for MockRunner {
    fn name(&self) -> Cow<'static, str> {
        "mock-runner".into()
    }

    /// The real run method
    async fn run(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
        _opt: &CommandRunOptions,
    ) -> anyhow::Result<ProcessOutput> {
        tracing::info!(%command, "Mock runner encountered command");
        let env = env.collect::<HashMap<_, _>>();
        let command = shellexpand::env_with_context_no_errors(command, |s| env.get(s));
        let cmd = self.input_output.get(command.as_ref());
        match cmd {
            Some(o) => {
                let mut out = o.clone();
                out.runned_inside = self.name().into_owned();
                Ok(out)
            }
            None => Err(anyhow::anyhow!(
                "This mock runner isn't configured to respond to command: {}",
                command
            )),
        }
    }
}
