use crate::prelude::FlowSnake;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message sent from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ServerMsg {
    #[serde(rename = "new_job")]
    NewJob(NewJob),
    #[serde(rename = "abort_job")]
    AbortJob(AbortJob),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub job: Job,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortJob {
    pub job_id: FlowSnake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: FlowSnake,
    pub repo: String,
    pub branch: Option<String>,
    pub test_suite: FlowSnake,
    pub test: Vec<String>,
    pub stage: JobStage,
    pub results: HashMap<String, TestResult>,
}

/// Message sent from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ClientMsg {
    #[serde(rename = "job_progress")]
    JobProgress(JobProgressMsg),

    #[serde(rename = "partial_result")]
    PartialResult(PartialResultMsg),

    #[serde(rename = "job_result")]
    JobResult(JobResultMsg),

    #[serde(rename = "client_status")]
    ClientStatus(ClientStatusMsg),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestResultKind {
    Accepted = 0,
    WrongAnswer = 1,
    RuntimeError = 2,
    PipelineFailed = 3,
    TimeLimitExceeded = 4,
    MemoryLimitExceeded = 5,
    NotRunned = -1,
    Waiting = -2,
    Running = -3,
    OtherError = -100,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JobStage {
    Queued,
    Dispatched,
    Fetching,
    Compiling,
    Running,
    Finished,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JobResultKind {
    Accepted,
    CompileError,
    PipelineError,
    JudgerError,
    Aborted,
    OtherError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub kind: TestResultKind,
    pub result_file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressMsg {
    pub id: FlowSnake,
    pub job_stage: JobStage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialResultMsg {
    pub job_id: FlowSnake,
    pub test_id: String,
    pub test_result: TestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultMsg {
    pub job_id: FlowSnake,
    pub job_result: JobResultKind,
    pub test_results: HashMap<String, TestResult>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusMsg {
    pub active_task_count: i32,
    pub can_accept_new_task: bool,
}
