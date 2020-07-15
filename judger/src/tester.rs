use super::judge::JobConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecErrorKind {
    RuntimeError(String),
    ReturnCodeCheckFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub ret_code: i16,
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMismatch {
    expected: String,
    got: String,
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
