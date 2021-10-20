use crate::{
    prelude::FlowSnake,
    runner::model::ProcessOutput,
    tester::model::{ExecErrorKind, JobFailure, SpjFailure},
};
use respector::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Message sent from server. See documentation on the server side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
#[serde(rename_all = "camelCase")]
pub enum ServerMsg {
    // Obsolete: NewJob
    #[serde(rename = "new_job_multi")]
    MultiNewJob(MultiNewJob),
    #[serde(rename = "abort_job")]
    AbortJob(AbortJob),
    #[serde(rename = "server_hello")]
    ServerHello,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiNewJob {
    pub reply_to: Option<FlowSnake>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbortJob {
    pub job_id: FlowSnake,
    pub as_cancel: bool,
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

/// Specification of a test suite, returned by the server.
///
/// This type is essentially the same as [`crate::tester::model::JudgerPublicConfig`],
/// but that type is the raw value stored test suite itself, while this is what gets stored
/// in the server's database.
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
    #[serde(rename = "receive_job")]
    ReceiveJob(ReceiveJobMsg),

    #[serde(rename = "job_progress")]
    JobProgress(JobProgressMsg),

    #[serde(rename = "partial_result")]
    PartialResult(PartialResultMsg),

    #[serde(rename = "job_output")]
    JobOutput(JobOutputMsg),

    #[serde(rename = "job_result")]
    JobResult(JobResultMsg),

    // Obsolete
    // #[serde(rename = "client_status")]
    // ClientStatus(ClientStatusMsg),
    //
    /// Requests some job from coordinator
    #[serde(rename = "job_request")]
    JobRequest(JobRequestMsg),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestResultKind {
    Accepted = 0,
    WrongAnswer = 1,
    RuntimeError = 2,
    PipelineFailed = 3,
    TimeLimitExceeded = 4,
    MemoryLimitExceeded = 5,
    ShouldFail = 6,
    NotRan = -1,
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
    Aborted,
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

pub type Score = Option<f64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub kind: TestResultKind,
    pub score: Score,
    pub result_file_id: Option<String>,
}

/// Represents the resulting score of a single test
pub trait ToScore {
    fn to_score(&self) -> Score;
}

impl ToScore for f64 {
    fn to_score(&self) -> Score {
        Some(*self)
    }
}

impl ToScore for () {
    fn to_score(&self) -> Score {
        None
    }
}

pub async fn transform_and_upload_test_result(
    failure: Result<impl ToScore, JobFailure>,
    output: Vec<ProcessOutput>,
    upload_info: Arc<ResultUploadConfig>,
    test_id: &str,
) -> TestResult {
    let score = failure.as_ref().ok().map_or(None, ToScore::to_score);
    let (result_kind, message, stdout_diff) = match failure {
        Ok(_) => (TestResultKind::Accepted, "".to_string().into(), None),
        Err(e) => match e {
            JobFailure::OutputMismatch(diff) => (
                TestResultKind::WrongAnswer,
                "The standard output of the program does not match the expected output."
                    .to_string()
                    .into(),
                Some(diff),
            ),
            JobFailure::SpjWrongAnswer(SpjFailure { diff, reason }) => {
                (TestResultKind::WrongAnswer, reason, diff)
            }
            JobFailure::ExecError(e) => match e.kind {
                ExecErrorKind::RuntimeError(err) => (TestResultKind::RuntimeError, Some(err), None),
                ExecErrorKind::ReturnCodeCheckFailed => (
                    TestResultKind::PipelineFailed,
                    Some("Some program's return code is not 0".into()),
                    None,
                ),
                ExecErrorKind::TimedOut => (
                    TestResultKind::TimeLimitExceeded,
                    Some("The user's program has exceeded its maximum execution time.".into()),
                    None,
                ),
            },
            JobFailure::InternalError(e) => (TestResultKind::OtherError, Some(e.to_string()), None),
            JobFailure::ShouldFail(_) => (
                TestResultKind::ShouldFail,
                Some("The tested program should return a non-zero number at some point".into()),
                None,
            ),
            JobFailure::Cancelled => (TestResultKind::NotRan, None, None),
        },
    };

    let output_file = JobOutputFile {
        output,
        stdout_diff,
        message,
    };

    let result_file_id = upload_test_result(output_file, upload_info, test_id).await;

    let result = TestResult {
        kind: result_kind,
        score,
        result_file_id,
    };

    result
}

pub async fn upload_test_result(
    f: JobOutputFile,
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
    let resp = post
        .and_then(|x| x.error_for_status())
        .inspect_err(|e| log::warn!("Failed to upload:\n{:?}", e))
        .ok()?;
    resp.text()
        .await
        .inspect_err(|e| log::warn!("Failed to upload:\n{:?}", e))
        .ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiveJobMsg {
    pub reject: bool,
    pub job_id: FlowSnake,
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
pub struct JobOutputMsg {
    pub job_id: FlowSnake,
    pub stream: Option<String>,
    pub error: Option<String>,
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
pub struct JobRequestMsg {
    pub active_task_count: u32,
    pub request_for_new_task: u32,
    pub message_id: Option<FlowSnake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutputFile {
    pub output: Vec<ProcessOutput>,
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
