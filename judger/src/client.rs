pub mod config;
pub mod model;
pub mod sink;

use crate::{
    client::model::JobResultKind,
    config::{JudgeToml, JudgerPublicConfig},
    fs::{self, JUDGE_FILE_NAME},
    prelude::*,
    tester::model::JudgerPrivateConfig,
    tester::model::TestSuiteOptions,
};
use anyhow::Result;
use async_trait::async_trait;
use config::SharedClientData;

use err_derive::Error;
use futures::{
    stream::{SplitSink, SplitStream},
    Sink, SinkExt, StreamExt,
};
use http::Method;
use model::*;
use serde::Serialize;
use serde_json::from_slice;
use sink::*;
use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::atomic::Ordering, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::info_span;
use tracing_futures::Instrument;

// Arc<Mutex<WsSink>>==>>Arc<WsSink>

#[derive(Debug, Error)]
pub enum JobExecErr {
    #[error(display = "No such file: {}", _0)]
    NoSuchFile(String),

    #[error(display = "No such config: {}", _0)]
    NoSuchConfig(String),

    #[error(display = "IO error: {}", _0)]
    Io(#[error(source)] std::io::Error),

    #[error(display = "Web request error: {}", _0)]
    Request(#[error(source)] reqwest::Error),

    #[error(display = "Websocket error: {}", _0)]
    Ws(#[error(source, no_from)] tokio_tungstenite::tungstenite::Error),

    #[error(display = "JSON error: {}", _0)]
    Json(#[error(source)] serde_json::Error),

    #[error(display = "TOML error: {}", _0)]
    TomlDes(#[error(source)] toml::de::Error),

    #[error(display = "Build error: {}", _0)]
    Build(#[error(source)] crate::tester::BuildError),

    #[error(display = "Execution error: {}", _0)]
    Exec(#[error(source)] crate::tester::ExecError),

    #[error(display = "Job was cancelled")]
    Cancelled,

    #[error(display = "Other error: {}", _0)]
    Any(anyhow::Error),
}

impl From<tokio_tungstenite::tungstenite::error::Error> for JobExecErr {
    fn from(e: tokio_tungstenite::tungstenite::error::Error) -> Self {
        match e {
            tokio_tungstenite::tungstenite::Error::Io(e) => JobExecErr::Io(e),
            _ => JobExecErr::Ws(e),
        }
    }
}

macro_rules! anyhow_downcast_chain {
    ($e:expr, $($ty:ty),*) => {
        $(if $e.is::<$ty>(){
            let e = $e.downcast::<$ty>().unwrap();
            return e.into();
        })*
    };
}

impl From<anyhow::Error> for JobExecErr {
    fn from(e: anyhow::Error) -> Self {
        anyhow_downcast_chain!(
            e,
            std::io::Error,
            crate::tester::BuildError,
            crate::tester::ExecError,
            toml::de::Error,
            reqwest::Error
        );
        JobExecErr::Any(e)
    }
}

#[derive(Debug, Error)]
pub enum ClientConnectionErr {
    #[error(display = "Websocket error: {}", _0)]
    Ws(#[error(from)] tokio_tungstenite::tungstenite::Error),
    #[error(display = "Bad access token")]
    BadAccessToken,
    #[error(display = "Bad register token")]
    BadRegisterToken,
}

/// Try to register at the coordinator if no access token was specified.
///
/// Returns `Ok(true)` if register was success, `Ok(false)` if register is not
/// needed or not applicable.
pub async fn try_register(cfg: &mut SharedClientData, refresh: bool) -> anyhow::Result<bool> {
    tracing::info!(
        "Registering judger. Access token: {:?}; Register token: {:?}",
        cfg.cfg.access_token,
        cfg.cfg.register_token
    );
    if (!refresh && cfg.cfg.access_token.is_some()) || cfg.cfg.register_token.is_none() {
        return Ok(false);
    }

    let req_body = JudgerRegisterMessage {
        token: cfg.cfg.register_token.clone().unwrap(),
        alternate_name: cfg.cfg.alternate_name.clone(),
        tags: cfg.cfg.tags.clone(),
    };
    let endpoint = cfg.register_endpoint();
    let client = &cfg.client;
    let res = client
        .request(Method::POST, &endpoint)
        .json(&req_body)
        .send()
        .await?;

    let status = res.status().as_u16();
    if status >= 300 {
        let headers = res.headers();
        tracing::error!("Failed to register judger. Status: {}", status);
        tracing::error!("Headers: {:#?}", headers);
        let body = res.text().await?;
        tracing::error!("body: {}", body);
        return Err(anyhow::Error::msg(format!(
            "Failed to register judger: status code {}",
            status
        )));
    }
    let res = res.text().await?;

    tracing::info!("Got new access token: {}", res);
    cfg.cfg.access_token = Some(res);

    Ok(true)
}

/// Verify if the current registration is active.
pub async fn verify_self(cfg: &SharedClientData) -> anyhow::Result<bool> {
    tracing::info!("Verifying access token {:?}", cfg.cfg.access_token);
    if cfg.cfg.access_token.is_none() {
        return Ok(false);
    }

    let endpoint = cfg.verify_endpoint();
    let res = cfg
        .client
        .request(Method::GET, &endpoint)
        .header("authorization", cfg.cfg.access_token.as_ref().unwrap())
        .send()
        .await?
        .status()
        .is_success();
    Ok(res)
}

pub async fn connect_to_coordinator(
    cfg: &SharedClientData,
) -> Result<(RawWsSink, WsStream), ClientConnectionErr> {
    let endpoint = cfg.websocket_endpoint();
    let req = http::Request::builder().uri(&endpoint);
    tracing::info!("Connecting to {}", endpoint);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    tracing::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

async fn fetch_test_suite_data(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<TestSuite, JobExecErr> {
    tracing::info!("Fetching data for test suite {}", suite_id);
    let suite_endpoint = cfg.test_suite_info_endpoint(suite_id);
    let res = cfg
        .client
        .get(&suite_endpoint)
        .send()
        .await?
        .json::<TestSuite>()
        .await?;
    Ok(res)
}

pub async fn check_download_read_test_suite(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<JudgerPublicConfig, JobExecErr> {
    tracing::info!("Checking test suite {}", suite_id);
    let suite_folder_root = cfg.test_suite_folder_root();
    tokio::fs::create_dir_all(suite_folder_root).await?;
    let suite_folder = cfg.test_suite_folder(suite_id);

    // My fault - The cancellation token should automagically cancel itself when
    // dropped in this case - If download fails then it won't cancel.

    /// This struct automatically releases the test suite inside it if dropped.
    ///
    /// TODO: Move this struct inside `SharedClientData`.
    struct AutoReleaseToken<'a>(CancellationTokenHandle, &'a SharedClientData, FlowSnake);
    impl<'a> Drop for AutoReleaseToken<'a> {
        fn drop(&mut self) {
            self.0.cancel();
            self.1.suite_unlock(self.2);
        }
    }

    tracing::debug!("Folder created: {:?}", suite_folder);
    let handle = cfg
        .obtain_suite_lock(suite_id)
        .instrument(info_span!("suite_lock", %suite_id))
        .await
        .map(|x| AutoReleaseToken(x, cfg, suite_id));

    // Lock this specific test suite and let all other concurrent tasks to wait
    // until downloading completes

    let suite_data = fetch_test_suite_data(suite_id, cfg).await?;

    let dir_exists = {
        let create_dir = tokio::fs::create_dir(&suite_folder).await;
        match create_dir {
            Ok(()) => false,
            Err(e) => match e.kind() {
                std::io::ErrorKind::AlreadyExists => true,
                _ => return Err(e.into()),
            },
        }
    };

    let lockfile = cfg.test_suite_folder_lockfile(suite_id);

    let lockfile_up_to_date = {
        let lockfile_data = tokio::fs::read_to_string(&lockfile).await;
        let lockfile_data = match lockfile_data {
            Ok(f) => Some(f),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => None,
                _ => return Err(e.into()),
            },
        };

        let suite_data_locked = lockfile_data
            .as_deref()
            .and_then(|x| serde_json::from_str::<TestSuite>(x).ok());

        suite_data_locked
            .map(|locked| locked.package_file_id == suite_data.package_file_id)
            .unwrap_or(false)
    };

    if !dir_exists || !lockfile_up_to_date {
        let endpoint = cfg.test_suite_download_endpoint(suite_id);
        let filename = cfg.random_temp_file_path();
        let file_folder_root = cfg.temp_file_folder_root();

        fs::ensure_removed_dir(&suite_folder).await?;
        tokio::fs::create_dir_all(file_folder_root).await?;
        tracing::info!(
            "Test suite does not exist. Initiating download of suite {} from {} to {:?}",
            suite_id,
            &endpoint,
            &filename
        );
        fs::net::download_unzip(
            cfg.client.clone(),
            cfg.client
                .get(&endpoint)
                .header("authorization", cfg.cfg.access_token.as_ref().unwrap())
                .build()?,
            &suite_folder,
            &filename,
        )
        .await?;
    }

    // Rewrite lockfile AFTER all data are saved
    if !lockfile_up_to_date {
        let serialized = serde_json::to_string(&suite_data)?;
        tokio::fs::write(&lockfile, &serialized).await?;
    }

    tracing::info!("Suite downloaded");

    // Note:
    // Lockfile is updated only AFTER test suite is fully downloaded, so an incomplete
    // download would not result in an updated lockfile. Therefore there's no need
    // to clean up the suite folder if things blow up - they're simply ignored.
    //
    // This should be easier to write using traditional try-catch-finally pattern
    // since finally-blocks can also be async. Sadly we don't have AsyncDrop trait
    // yet here in Rust. See this withoutboats' post for more information:
    // <https://without.boats/blog/poll-drop/>
    //
    //   |
    //   V
    // let _ = fs::ensure_removed_dir(&cfg.test_suite_folder(suite_id)).await;

    // The handle should be dropped right here
    drop(handle);

    let mut judger_conf_dir = suite_folder.clone();
    judger_conf_dir.push("testconf.json");
    let judger_conf = match tokio::fs::read(&judger_conf_dir).await {
        Ok(c) => c,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                return Err(JobExecErr::NoSuchFile(
                    judger_conf_dir.to_string_lossy().to_owned().to_string(),
                ));
            }
            _ => return Err(JobExecErr::Io(e)),
        },
    };
    let judger_conf = serde_json::from_slice::<JudgerPublicConfig>(&judger_conf)?;

    Ok(judger_conf)
}

pub async fn handle_job_wrapper(
    job: NewJob,
    send: Arc<WsSink>,
    cancel: CancellationTokenHandle,
    cfg: Arc<SharedClientData>,
) {
    // TODO: Handle failed cases and report
    let job_id = job.job.id;
    flag_new_job(send.clone(), cfg.clone()).await;
    let msg = match handle_job(job, send.clone(), cancel, cfg.clone())
        .instrument(tracing::info_span!("handle_job", %job_id))
        .await
    {
        Ok(_res) => ClientMsg::JobResult(_res),
        Err(err) => {
            tracing::warn!("job {} aborted because of error: {:?}", job_id, &err);

            let (err, msg) = match err {
                JobExecErr::NoSuchFile(f) => (
                    JobResultKind::CompileError,
                    format!("Cannot find file: {}", f),
                ),
                JobExecErr::NoSuchConfig(f) => (
                    JobResultKind::CompileError,
                    format!("Cannot find config for {} in `judger.toml`", f),
                ),
                JobExecErr::Io(e) => (JobResultKind::JudgerError, format!("IO error: {:?}", e)),
                JobExecErr::Ws(e) => (
                    JobResultKind::JudgerError,
                    format!("Websocket error: {:?}", e),
                ),
                JobExecErr::Cancelled => (JobResultKind::Aborted, "Job cancelled by user".into()),
                JobExecErr::Json(e) => (JobResultKind::JudgerError, format!("JSON error: {:?}", e)),
                JobExecErr::TomlDes(e) => (
                    JobResultKind::JudgerError,
                    format!("TOML deserialization error: {:?}", e),
                ),
                JobExecErr::Request(e) => (
                    JobResultKind::JudgerError,
                    format!("Web request error: {:?}", e),
                ),
                JobExecErr::Build(e) => (JobResultKind::CompileError, format!("{}", e)),
                JobExecErr::Exec(e) => (JobResultKind::PipelineError, format!("{:?}", e)),
                JobExecErr::Any(e) => (JobResultKind::OtherError, format!("{:?}", e)),
            };

            ClientMsg::JobResult(JobResultMsg {
                job_id,
                results: HashMap::new(),
                job_result: err,
                message: Some(msg),
            })
        }
    };

    while let Err(e) = {
        // Ah yes, do-while pattern
        let mut req = cfg.client.post(&cfg.result_send_endpoint()).json(&msg);
        if let Some(token) = &cfg.cfg.access_token {
            req = req.header("authorization", token.as_str());
        }
        req.send().await.and_then(|x| x.error_for_status())
    } {
        tracing::error!("Error when sending job result mesage:\n{}", e)
    }

    flag_finished_job(send.clone(), cfg.clone()).await;

    tracing::info!("{}: Result message sent", job_id);

    {
        cfg.running_job_handles.lock().await.remove(&job_id);
    }

    match fs::ensure_removed_dir(&cfg.job_folder(job_id)).await {
        Ok(_) => {}
        Err(e) => tracing::error!("Failed to remove directory for job {}: {}", job_id, e),
    };
    tracing::info!("{}: cleanup complete", job_id);
}

pub async fn handle_job(
    job: NewJob,
    send: Arc<WsSink>,
    cancel: CancellationTokenHandle,
    cfg: Arc<SharedClientData>,
) -> Result<JobResultMsg, JobExecErr> {
    let job = job.job;
    let client = reqwest::Client::new();

    tracing::info!("created");

    let mut public_cfg = check_download_read_test_suite(job.test_suite, &*cfg)
        .with_cancel(cancel.clone())
        .instrument(info_span!("download_test_suites", %job.test_suite))
        .await
        .ok_or(JobExecErr::Cancelled)??;
    public_cfg.binds.get_or_insert_with(Vec::new);
    tracing::info!("got test suite");

    send.send_msg(&ClientMsg::JobProgress(JobProgressMsg {
        job_id: job.id,
        stage: JobStage::Fetching,
    }))
    .await?;

    // Clone the repo specified in job
    let job_path = cfg.job_folder(job.id);
    let _ = fs::ensure_removed_dir(&job_path).await;
    fs::net::git_clone(
        &job_path,
        fs::net::GitCloneOptions {
            repo: job.repo,
            revision: job.revision,
            depth: 3,
        },
    )
    .await?;

    tracing::info!("fetched");

    let job_path: PathBuf = fs::find_judge_root(&job_path).await?;
    let mut judge_cfg = job_path.clone();
    judge_cfg.push(JUDGE_FILE_NAME);

    tracing::info!("found job description file at {:?}", &judge_cfg);

    let judge_cfg = tokio::fs::read(judge_cfg).await?;
    let judge_cfg = toml::from_slice::<JudgeToml>(&judge_cfg)?;

    tracing::info!("read job description file");

    let judge_job_cfg = judge_cfg
        .jobs
        .get(&public_cfg.name)
        .ok_or_else(|| JobExecErr::NoSuchConfig(public_cfg.name.to_owned()))?;

    let image = judge_job_cfg.image.clone();

    tracing::info!("prepare to run");

    send.send_msg(&ClientMsg::JobProgress(JobProgressMsg {
        job_id: job.id,
        stage: JobStage::Running,
    }))
    .await?;

    // Set run script
    let run = judge_job_cfg
        .run
        .iter()
        .chain(public_cfg.run.iter())
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    public_cfg.run = run;

    let suite_root_path = cfg.test_suite_folder(job.test_suite);
    let mut tests_path = suite_root_path.clone();
    tests_path.push(&public_cfg.mapped_dir.from);
    let private_cfg = JudgerPrivateConfig {
        test_root_dir: tests_path,
        mapped_test_root_dir: public_cfg.mapped_dir.to.clone(),
    };

    let options = TestSuiteOptions {
        tests: job.tests.clone(),
        time_limit: public_cfg.time_limit.map(|x| x as usize),
        mem_limit: public_cfg.memory_limit.map(|x| x as usize),
        build_image: true,
        remove_image: true,
    };

    let mut suite = crate::tester::exec::TestSuite::from_config(
        image,
        &suite_root_path,
        private_cfg,
        public_cfg,
        options,
    )
    .await?;

    tracing::info!("options created");
    let (ch_send, ch_recv) = tokio::sync::mpsc::unbounded_channel();

    let recv_handle = tokio::spawn({
        let mut recv = ch_recv;
        let ws_send = send.clone();
        let job_id = job.id;
        async move {
            while let Some((key, res)) = recv.recv().await {
                tracing::info!("Job {}: recv message for key={}", job_id, key);
                // Omit error; it doesn't matter
                let _ = ws_send
                    .send_msg(&ClientMsg::PartialResult(PartialResultMsg {
                        job_id,
                        test_id: key,
                        test_result: res,
                    }))
                    .await;
            }
        }
    });

    let (build_ch_send, build_ch_recv) =
        tokio::sync::mpsc::unbounded_channel::<bollard::models::BuildInfo>();

    let build_recv_handle = tokio::spawn({
        let mut recv = build_ch_recv;
        let ws_send = send.clone();
        let job_id = job.id;
        async move {
            while let Some(res) = recv.recv().await {
                let _ = ws_send
                    .send_msg(&ClientMsg::JobOutput(JobOutputMsg {
                        job_id,
                        stream: res.stream,
                        error: res.error,
                    }))
                    .await;
            }
        }
    });

    let docker = bollard::Docker::connect_with_local_defaults().unwrap();

    tracing::info!("started.");

    let upload_info = Arc::new(ResultUploadConfig {
        client,
        endpoint: cfg.result_upload_endpoint(),
        access_token: cfg.cfg.access_token.clone(),
        job_id: job.id,
    });

    let result = suite
        .run(
            docker,
            job_path,
            Some(build_ch_send),
            Some(ch_send),
            Some(upload_info),
            cancel.clone(),
        )
        .instrument(info_span!("run_job"))
        .await?;

    tracing::info!("finished running");

    let _ = build_recv_handle.await;
    let _ = recv_handle.await;

    tracing::info!("finished");

    let job_result = JobResultMsg {
        job_id: job.id,
        results: result,
        job_result: JobResultKind::Accepted,
        message: None,
    };
    Ok(job_result)
}

pub async fn flag_new_job(send: Arc<WsSink>, client_config: Arc<SharedClientData>) {
    let job_count = client_config.new_job();
    let can_accept_new_task =
        client_config.running_tests.load(Ordering::SeqCst) < client_config.cfg.max_concurrent_tasks;

    let _ = send
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: job_count as i32,
            request_for_new_task: 0,
            can_accept_new_task,
        }))
        .await;
}

pub async fn flag_finished_job(send: Arc<WsSink>, client_config: Arc<SharedClientData>) {
    let job_count = client_config.finish_job();
    let aborting = client_config.aborting.load(Ordering::SeqCst);
    let _ = send
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: job_count as i32,
            can_accept_new_task: !aborting,
            request_for_new_task: if aborting { 0 } else { 1 },
        }))
        .await;
}

pub async fn accept_job(job: NewJob, send: Arc<WsSink>, client_config: Arc<SharedClientData>) {
    tracing::info!("Received job {}", job.job.id);
    let job_id = job.job.id;
    let cancel_handle = client_config.cancel_handle.child_token();
    let cancel_token = cancel_handle.child_token();
    let handle = tokio::spawn(handle_job_wrapper(
        job,
        send,
        cancel_token,
        client_config.clone(),
    ));
    client_config
        .running_job_handles
        .lock()
        .await
        .insert(job_id, (handle, cancel_handle));
}

async fn cancel_job(job_id: FlowSnake, client_config: Arc<SharedClientData>) {
    let job = {
        client_config
            .running_job_handles
            .lock()
            .await
            .remove(&job_id)
    };
    if let Some((handle, cancel)) = job {
        cancel.cancel();
        match handle.await {
            Ok(_) => tracing::info!("Cancelled job {}", job_id),
            Err(e) => tracing::warn!("Unable to cancel job {}: {}", job_id, e),
        };
    }
    // remove self from cancelling job list
    client_config
        .cancelling_job_handles
        .lock()
        .await
        .remove(&job_id);
}

async fn keepalive(
    client_config: Arc<SharedClientData>,
    keepalive_token: CancellationTokenHandle,
    ws: Arc<WsSink>,
    interval: std::time::Duration,
) {
    while tokio::time::sleep(interval)
        .with_cancel(client_config.cancel_handle.child_token())
        .await
        .is_some()
    {
        match {
            ws.send_conf(tokio_tungstenite::tungstenite::Message::Ping(vec![]), true)
                .await
        } {
            Ok(_) => {}
            Err(e) => {
                keepalive_token.cancel();
                tracing::error!("Server disconnected: {}", e);
                break;
            }
        };
    }
}

#[allow(clippy::if_same_then_else)]
pub async fn client_loop(
    mut ws_recv: WsStream,
    ws_send: Arc<WsSink>,
    client_config: Arc<SharedClientData>,
) -> Arc<WsSink> {
    // Request for max_concurrent_tasks jobs
    ws_send
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: 0,
            request_for_new_task: client_config.cfg.max_concurrent_tasks as u32,
            can_accept_new_task: true,
        }))
        .await
        .unwrap();

    let keepalive_token = client_config.cancel_handle.child_token();
    let keepalive_cancel = keepalive_token.child_token();

    let keepalive_handle = tokio::spawn(keepalive(
        client_config.clone(),
        keepalive_token,
        ws_send.clone(),
        std::time::Duration::from_secs(20),
    ));

    while let Some(Some(Ok(x))) = ws_recv
        .next()
        .with_cancel(client_config.cancel_handle.child_token())
        .await
    {
        let x: Message = x;
        if x.is_text() {
            let payload = x.into_data();
            let msg = from_slice::<ServerMsg>(&payload);
            match msg {
                Ok(msg) => match msg {
                    ServerMsg::NewJob(job) => {
                        accept_job(job, ws_send.clone(), client_config.clone()).await
                    }
                    ServerMsg::AbortJob(job) => {
                        let abort = tokio::spawn(cancel_job(job.job_id, client_config.clone()));
                        client_config
                            .cancelling_job_handles
                            .lock()
                            .await
                            .insert(job.job_id, abort);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Unable to deserialize mesage: {}\nError: {:?}",
                        String::from_utf8_lossy(&payload),
                        e
                    );
                }
            }
        } else if x.is_ping() {
            // noop
        } else if x.is_pong() {
            // also noop
        } else {
            tracing::warn!("Unsupported message: {:?}", x);
        }
    }
    ws_send.clear_socket();

    let _ = keepalive_handle.await;

    tracing::warn!("Disconnected!");
    ws_send
}
