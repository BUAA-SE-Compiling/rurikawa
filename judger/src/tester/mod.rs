pub mod exec;
pub mod model;
pub mod runner;
pub mod utils;
pub mod spj;

use err_derive::Error;
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
    pub diff: String,
    pub output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Error)]
#[error(
    display = "Execution error in stage {}: {:?};\noutputs: {:?}",
    stage,
    kind,
    output
)]
pub struct ExecError {
    pub stage: usize,
    pub kind: ExecErrorKind,
    pub output: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ShouldFailFailure {
    pub output: Vec<ProcessInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuildError {
    ImagePullFailure(String),
    FileTransferError(String),
    BuildError {
        error: String,
        detail: Option<bollard::models::ErrorDetail>,
    },
    Internal(String),
    Cancelled,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for BuildError {}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum JobFailure {
    OutputMismatch(OutputMismatch),
    ExecError(ExecError),
    InternalError(String),
    ShouldFail(ShouldFailFailure),
    Cancelled,
}

impl std::fmt::Display for JobFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for JobFailure {}

impl JobFailure {
    /// Make a new `InternalError`, the lazy way.
    pub fn internal_err_from<D>(error: D) -> JobFailure
    where
        D: std::fmt::Display,
    {
        JobFailure::InternalError(format!("{}", error))
    }
}
