pub mod diff;
pub mod exec;

use super::judge::JobConfig;
use serde::{Deserialize, Serialize};
use subprocess::{CaptureData, ExitStatus};

pub use subprocess;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecErrorKind {
    RuntimeError(String),
    ReturnCodeCheckFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub ret_code: i64,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
}

impl From<(String, CaptureData)> for ProcessInfo {
    fn from(pair: (String, CaptureData)) -> Self {
        let (
            command,
            CaptureData {
                stdout,
                stderr,
                exit_status,
            },
        ) = pair;
        let stdout = String::from_utf8(stdout).unwrap();
        let stderr = String::from_utf8(stderr).unwrap();
        ProcessInfo {
            command,
            stdout,
            stderr,
            ret_code: match exit_status {
                ExitStatus::Exited(x) => x as i64,
                ExitStatus::Signaled(x) => -(x as i64),
                ExitStatus::Other(x) => x as i64,
                ExitStatus::Undetermined => 1,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMismatch {
    diff: String,
    output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecError {
    stage: usize,
    kind: ExecErrorKind,
    output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobFailure {
    OutputMismatch(OutputMismatch),
    ExecError(ExecError),
}

pub fn run_job(job: &JobConfig) -> Result<(), JobFailure> {
    Ok(())
}
