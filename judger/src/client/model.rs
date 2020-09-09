use crate::prelude::FlowSnake;
use bytes::buf::BufMutExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message sent from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
#[serde(rename_all = "camelCase")]
pub enum ServerMsg {
    #[serde(rename = "new_job")]
    NewJob(NewJob),
    #[serde(rename = "abort_job")]
    AbortJob(AbortJob),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewJob {
    pub job: Job,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbortJob {
    pub job_id: FlowSnake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: FlowSnake,
    pub repo: String,
    pub branch: Option<String>,
    pub test_suite: FlowSnake,
    pub tests: Vec<String>,
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
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub kind: TestResultKind,
    pub result_file_id: Option<String>,
}

pub async fn upload_test_result(
    f: Result<(), crate::tester::JobFailure>,
    upload_info: Option<(&str, reqwest::Client)>,
) -> TestResult {
    match f {
        Ok(_) => TestResult {
            kind: TestResultKind::Accepted,
            result_file_id: None,
        },
        Err(e) => {
            let (kind, cache) = match e {
                crate::tester::JobFailure::OutputMismatch(m) => (
                    TestResultKind::WrongAnswer,
                    FailedJobOutputCacheFile {
                        output: m.output,
                        stdout_diff: Some(m.diff),
                        message: None,
                    },
                ),

                crate::tester::JobFailure::ExecError(e) => {
                    let (res, msg) = match e.kind {
                        crate::tester::ExecErrorKind::RuntimeError(e) => {
                            (TestResultKind::RuntimeError, Some(e))
                        }
                        crate::tester::ExecErrorKind::ReturnCodeCheckFailed => (
                            TestResultKind::PipelineFailed,
                            Some("Return code check failed".into()),
                        ),
                        crate::tester::ExecErrorKind::TimedOut => {
                            (TestResultKind::TimeLimitExceeded, None)
                        }
                    };
                    (
                        res,
                        FailedJobOutputCacheFile {
                            output: e.output,
                            stdout_diff: None,
                            message: msg,
                        },
                    )
                }
                crate::tester::JobFailure::InternalError(e) => (
                    TestResultKind::OtherError,
                    FailedJobOutputCacheFile {
                        output: Vec::new(),
                        stdout_diff: None,
                        message: Some(e),
                    },
                ),
            };

            let result_file_id = if let Some((upload_endpoint, client)) = upload_info {
                let post = client.post(upload_endpoint).json(&cache).send().await.ok();
                if let Some(resp) = post {
                    resp.text().await.ok()
                } else {
                    None
                }
            } else {
                None
            };
            TestResult {
                kind,
                result_file_id,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressMsg {
    pub job_id: FlowSnake,
    pub job_stage: JobStage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialResultMsg {
    pub job_id: FlowSnake,
    pub test_id: String,
    pub test_result: TestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResultMsg {
    pub job_id: FlowSnake,
    pub job_result: JobResultKind,
    pub test_results: HashMap<String, TestResult>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatusMsg {
    pub active_task_count: i32,
    pub can_accept_new_task: bool,
    pub request_for_new_task: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedJobOutputCacheFile {
    pub output: Vec<crate::tester::ProcessInfo>,
    pub stdout_diff: Option<String>,
    pub message: Option<String>,
}
