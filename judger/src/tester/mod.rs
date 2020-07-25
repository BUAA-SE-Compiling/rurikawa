pub mod exec;
pub mod runner;
pub mod utils;

use super::judge::JobConfig;
use crate::tester::runner::DockerCommandRunner;
use exec::{Capturable, Step, Test};
use names::{Generator, Name};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ExecErrorKind {
    RuntimeError(String),
    ReturnCodeCheckFailed,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProcessInfo {
    pub ret_code: i32,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct OutputMismatch {
    diff: String,
    output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ExecError {
    stage: usize,
    kind: ExecErrorKind,
    output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum JobFailure {
    OutputMismatch(OutputMismatch),
    ExecError(ExecError),
}

impl JobConfig {
    pub async fn run(&self) -> Result<(), JobFailure> {
        // TODO: Use the mem_limit field
        let mut names = Generator::with_naming(Name::Numbered);
        let mut runner = DockerCommandRunner::new(
            bollard::Docker::connect_with_unix_defaults().unwrap(),
            &dbg!(names.next().unwrap()),
            &self.image_name,
        )
        .await;
        let mut t = Test::new();
        for step in self.before_exec.clone() {
            t.add_step(Step::new(Capturable::new(step)));
        }
        t.add_step(Step::new(Capturable::new(self.exec.clone())));
        t.expected(&self.expected_out);
        t.run(&mut runner).await?;
        Ok(())
    }
}
