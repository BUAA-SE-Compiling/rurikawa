use crate::{prelude::FlowSnake, tester::exec::Bind};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

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
    pub revision: String,
    pub test_suite: FlowSnake,
    pub tests: Vec<String>,
    pub stage: JobStage,
    pub results: HashMap<String, TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSuite {
    pub id: FlowSnake,
    pub name: String,
    pub title: String,
    pub description: String,
    pub tags: Option<Vec<String>>,
    pub package_file_id: String,

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

#[derive(Debug)]
pub struct ResultUploadConfig {
    pub client: reqwest::Client,
    pub endpoint: String,
    pub access_token: Option<String>,
    pub job_id: FlowSnake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub kind: TestResultKind,
    pub result_file_id: Option<String>,
}

impl TestResult {
    pub fn from_failure(
        failure: Result<(), crate::tester::JobFailure>,
    ) -> (TestResult, Option<FailedJobOutputCacheFile>) {
        match failure {
            Ok(_) => (
                TestResult {
                    kind: TestResultKind::Accepted,
                    result_file_id: None,
                },
                None,
            ),
            Err(e) => {
                let (kind, cache) = match e {
                    crate::tester::JobFailure::OutputMismatch(m) => (
                        TestResultKind::WrongAnswer,
                        Some(FailedJobOutputCacheFile {
                            output: m.output,
                            stdout_diff: Some(m.diff),
                            message: None,
                        }),
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
                            Some(FailedJobOutputCacheFile {
                                output: e.output,
                                stdout_diff: None,
                                message: msg,
                            }),
                        )
                    }
                    crate::tester::JobFailure::InternalError(e) => (
                        TestResultKind::OtherError,
                        Some(FailedJobOutputCacheFile {
                            output: Vec::new(),
                            stdout_diff: None,
                            message: Some(e),
                        }),
                    ),
                    crate::tester::JobFailure::Cancelled => (TestResultKind::NotRunned, None),
                };

                (
                    TestResult {
                        kind,
                        result_file_id: None,
                    },
                    cache,
                )
            }
        }
    }
}

pub async fn upload_test_result(
    f: FailedJobOutputCacheFile,
    upload_info: Arc<ResultUploadConfig>,
    test_id: &str,
) -> Option<String> {
    let mut post = upload_info.client.post(&upload_info.endpoint);
    if let Some(hdr) = upload_info.access_token.as_ref() {
        post = post.header("authorization", hdr);
    }
    let post = post
        .query(&[
            ("jobId", upload_info.job_id.to_string().as_str()),
            ("testId", test_id),
        ])
        .json(&f)
        .send()
        .await;
    let resp = post.and_then(|x| x.error_for_status());
    match resp {
        Ok(resp) => {
            let resp = resp.text().await;
            match resp {
                Ok(t) => Some(t),
                Err(e) => {
                    log::warn!("Failed to upload:\n{:?}", e);
                    None
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to upload:\n{:?}", e);
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressMsg {
    pub job_id: FlowSnake,
    pub stage: JobStage,
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
    pub results: HashMap<String, TestResult>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JudgerRegisterMessage {
    pub token: String,
    pub alternate_name: Option<String>,
    pub tags: Option<Vec<String>>,
}
