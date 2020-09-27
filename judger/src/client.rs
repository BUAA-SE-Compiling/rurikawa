pub mod model;

use crate::{
    client::model::JobResultKind,
    config::{JudgeToml, JudgerPublicConfig},
    fs::{self, JUDGE_FILE_NAME},
    prelude::*,
    tester::exec::JudgerPrivateConfig,
};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{
    stream::{SplitSink, SplitStream},
    FutureExt, Sink, SinkExt, StreamExt,
};
use http::Method;
use model::*;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::{
    collections::HashMap,
    fmt::Debug,
    path::PathBuf,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::{net::TcpStream, sync::Mutex, sync::RwLock, task::JoinHandle};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

pub type WsDuplex = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsDuplex, Message>;
pub type WsStream = SplitStream<WsDuplex>;

#[async_trait]
pub trait SendJsonMessage<M, T>
where
    T: Sink<Message> + Unpin + Send + Sync,
    M: Serialize,
{
    type Error;
    async fn send_msg(&mut self, msg: &M) -> Result<(), Self::Error>;
    // async fn send_msg_all<'a, I>(&mut self, msgs: I) -> Result<(), Self::Error>
    // where
    //     I: Stream<Item = &'a M> + Unpin + Send + Sync,
    //     M: 'a;
}

#[async_trait]
impl<M, T> SendJsonMessage<M, T> for T
where
    T: Sink<Message> + Unpin + Send + Sync,
    M: Serialize + Sync + Debug,
{
    type Error = T::Error;
    async fn send_msg(&mut self, msg: &M) -> Result<(), Self::Error> {
        log::info!("sent: {:?}", msg);
        let serialized = serde_json::to_string(msg).unwrap();
        let msg = Message::text(serialized);
        self.send(msg).await
    }

    // async fn send_msg_all<'a, I>(&mut self, msgs: I) -> Result<(), Self::Error>
    // where
    //     I: Stream<Item = &'a M> + Unpin + Send + Sync,
    //     M: 'a,
    // {
    //     self.send_all(&mut msgs.filter_map(|x| async {
    //         let serialized = serde_json::to_string(x).ok()?;
    //         Some(Message::text(serialized))
    //     }))
    //     .await
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub host: String,
    pub max_concurrent_tasks: usize,
    pub ssl: bool,
    pub access_token: Option<String>,
    pub register_token: Option<String>,
    pub alternate_name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub cache_folder: PathBuf,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            host: "".into(),
            max_concurrent_tasks: 1,
            ssl: false,
            access_token: None,
            register_token: None,
            alternate_name: None,
            tags: None,
            cache_folder: PathBuf::new(),
        }
    }
}

#[derive(Debug)]
pub struct SharedClientData {
    pub cfg: ClientConfig,
    pub running_tests: AtomicUsize,
    pub client: reqwest::Client,
    /// All test suites whose folder is being edited.
    pub locked_test_suite: RwLock<HashMap<FlowSnake, Arc<Mutex<()>>>>,
    pub running_job_handles: Mutex<HashMap<FlowSnake, (JoinHandle<()>, CancellationTokenHandle)>>,
    pub cancelling_job_handles: Mutex<HashMap<FlowSnake, JoinHandle<()>>>,
    pub cancel_handle: CancellationTokenHandle,
}

impl SharedClientData {
    pub fn new(cfg: ClientConfig) -> SharedClientData {
        SharedClientData {
            cfg,
            client: reqwest::Client::new(),
            running_tests: AtomicUsize::new(0),
            locked_test_suite: RwLock::new(HashMap::new()),
            running_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_handles: Mutex::new(HashMap::new()),
            cancel_handle: CancellationTokenHandle::new(),
        }
    }

    pub fn register_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/register", ssl, self.cfg.host)
    }

    pub fn verify_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/verify", ssl, self.cfg.host)
    }

    pub fn websocket_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("wss")
        } else {
            format_args!("ws")
        };

        if let Some(token) = &self.cfg.access_token {
            format!(
                "{}://{}/api/v1/judger/ws?token={}",
                ssl, self.cfg.host, token
            )
        } else {
            format!("{}://{}/api/v1/judger/ws", ssl, self.cfg.host)
        }
    }

    pub fn test_suite_download_endpoint(&self, suite_id: FlowSnake) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!(
            "{}://{}/api/v1/judger/download-suite/{}",
            ssl, self.cfg.host, suite_id
        )
    }

    pub fn result_upload_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/judger/upload", ssl, self.cfg.host)
    }

    pub fn job_folder_root(&self) -> PathBuf {
        let mut job_temp_folder = self.cfg.cache_folder.clone();
        job_temp_folder.push("jobs");
        job_temp_folder
    }

    pub fn test_suite_folder_root(&self) -> PathBuf {
        let mut test_suite_temp_folder = self.cfg.cache_folder.clone();
        test_suite_temp_folder.push("suites");
        test_suite_temp_folder
    }

    pub fn job_folder(&self, job_id: FlowSnake) -> PathBuf {
        let mut job_temp_folder = self.job_folder_root();
        job_temp_folder.push(job_id.to_string());
        job_temp_folder
    }

    pub fn test_suite_folder(&self, suite_id: FlowSnake) -> PathBuf {
        let mut test_suite_temp_folder = self.test_suite_folder_root();
        test_suite_temp_folder.push(suite_id.to_string());
        test_suite_temp_folder
    }

    pub fn temp_file_folder_root(&self) -> PathBuf {
        let mut test_suite_temp_folder = self.cfg.cache_folder.clone();
        test_suite_temp_folder.push("files");
        test_suite_temp_folder
    }

    pub fn random_temp_file_path(&self) -> PathBuf {
        let mut root = self.temp_file_folder_root();
        let random_filename = FlowSnake::generate().to_string();
        root.push(random_filename);
        root
    }

    pub async fn obtain_suite_lock(&self, suite_id: FlowSnake) -> Arc<Mutex<()>> {
        let cur = self.locked_test_suite.read().await.get(&suite_id).cloned();
        if let Some(cur) = cur {
            cur
        } else {
            let arc = Arc::new(Mutex::new(()));
            self.locked_test_suite
                .write()
                .await
                .insert(suite_id, arc.clone());
            arc
        }
    }

    pub async fn suite_unlock(&self, suite_id: FlowSnake) {
        self.locked_test_suite.write().await.remove(&suite_id);
    }

    pub fn new_job(&self) -> usize {
        self.running_tests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn finish_job(&self) -> usize {
        self.running_tests
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum JobExecErr {
    NoSuchFile(String),
    NoSuchConfig(String),
    Io(std::io::Error),
    Ws(tungstenite::Error),
    Json(serde_json::Error),
    TomlDes(toml::de::Error),
    Exec(crate::tester::ExecError),
    Cancelled,
    Any(anyhow::Error),
}

impl From<std::io::Error> for JobExecErr {
    fn from(e: std::io::Error) -> Self {
        JobExecErr::Io(e)
    }
}

impl From<anyhow::Error> for JobExecErr {
    fn from(e: anyhow::Error) -> Self {
        JobExecErr::Any(e)
    }
}

impl From<crate::tester::ExecError> for JobExecErr {
    fn from(e: crate::tester::ExecError) -> Self {
        JobExecErr::Exec(e)
    }
}

impl From<serde_json::Error> for JobExecErr {
    fn from(e: serde_json::Error) -> Self {
        JobExecErr::Json(e)
    }
}

impl From<tungstenite::error::Error> for JobExecErr {
    fn from(e: tungstenite::error::Error) -> Self {
        match e {
            tungstenite::Error::Io(e) => JobExecErr::Io(e),
            _ => JobExecErr::Ws(e),
        }
    }
}

impl From<toml::de::Error> for JobExecErr {
    fn from(e: toml::de::Error) -> Self {
        JobExecErr::TomlDes(e)
    }
}

#[derive(Debug)]
pub enum ClientConnectionErr {
    Ws(tungstenite::Error),
    BadAccessToken,
    BadRegisterToken,
}

impl From<tungstenite::Error> for ClientConnectionErr {
    fn from(x: tungstenite::Error) -> ClientConnectionErr {
        ClientConnectionErr::Ws(x)
    }
}

/// Try to register at the coordinator if no access token was specified.
///
/// Returns `Ok(true)` if register was success, `Ok(false)` if register is not
/// needed or not applicable.
pub async fn try_register(cfg: &mut SharedClientData, refresh: bool) -> anyhow::Result<bool> {
    log::info!(
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
        .await?
        .error_for_status()?
        .text()
        .await?;

    log::info!("Got new access token: {}", res);
    cfg.cfg.access_token = Some(res);

    Ok(true)
}

/// Verify if the current registration is active.
pub async fn verify_self(cfg: &SharedClientData) -> anyhow::Result<bool> {
    log::info!("Verifying access token {:?}", cfg.cfg.access_token);
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
) -> Result<(WsSink, WsStream), ClientConnectionErr> {
    let endpoint = cfg.websocket_endpoint();
    let req = http::Request::builder().uri(&endpoint);
    log::info!("Connecting to {}", endpoint);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    log::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

pub async fn check_download_read_test_suite(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<JudgerPublicConfig, JobExecErr> {
    log::info!("Checking test suite {}", suite_id);
    let suite_folder_root = cfg.test_suite_folder_root();
    tokio::fs::create_dir_all(suite_folder_root).await?;
    let suite_folder = cfg.test_suite_folder(suite_id);
    {
        // Lock this specific test suite and let all other concurrent tasks to wait
        // until downloading completes
        let lock = cfg.obtain_suite_lock(suite_id);
        lock.await.lock().await;

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
        if !dir_exists {
            let endpoint = cfg.test_suite_download_endpoint(suite_id);
            let filename = cfg.random_temp_file_path();
            let file_folder_root = cfg.temp_file_folder_root();
            tokio::fs::create_dir_all(file_folder_root).await?;
            log::info!(
                "Test suite does not exits. Initiating download of suite {} from {} to {:?}",
                suite_id,
                &endpoint,
                &filename
            );
            fs::net::download_unzip(&endpoint, &suite_folder, &filename).await?;
        }
        log::info!("Suite downloaded");
        cfg.suite_unlock(suite_id).await;
    }

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
    send: Arc<Mutex<WsSink>>,
    cancel: CancellationToken,
    cfg: Arc<SharedClientData>,
) {
    // TODO: Handle failed cases and report
    let job_id = job.job.id;
    flag_new_job(send.clone(), cfg.clone()).await;
    match handle_job(job, send.clone(), cancel, cfg.clone()).await {
        Ok(_res) => {
            let send_res = send
                .lock()
                .await
                .send_msg(&ClientMsg::JobResult(_res))
                .await;
            match send_res {
                Ok(_) => {}
                Err(e) => log::error!("Error when sending job result mesage:\n{}", e),
            }
        }
        Err(_err) => {
            let (err, msg) = match _err {
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
                JobExecErr::Exec(e) => (JobResultKind::PipelineError, format!("{:?}", e)),
                JobExecErr::Any(e) => (JobResultKind::OtherError, format!("{:?}", e)),
            };
            let send_res = send
                .lock()
                .await
                .send_msg(&ClientMsg::JobResult(JobResultMsg {
                    job_id,
                    results: HashMap::new(),
                    job_result: err,
                    message: Some(msg),
                }))
                .await;
            match send_res {
                Ok(_) => {}
                Err(e) => log::error!("Error when sending job result mesage:\n{}", e),
            }
        }
    }
    flag_finished_job(send.clone(), cfg.clone()).await;

    match fs::ensure_removed_dir(&cfg.job_folder(job_id)).await {
        Ok(_) => {}
        Err(e) => log::error!("Failed to remove directory for job {}: {}", job_id, e),
    };
}

pub async fn handle_job(
    job: NewJob,
    send: Arc<Mutex<WsSink>>,
    cancel: CancellationToken,
    cfg: Arc<SharedClientData>,
) -> Result<JobResultMsg, JobExecErr> {
    let job = job.job;
    let client = reqwest::Client::new();

    log::info!("Job {}: created", job.id);

    let mut public_cfg = check_download_read_test_suite(job.test_suite, &*cfg)
        .with_cancel(cancel.clone())
        .await
        .ok_or(JobExecErr::Cancelled)??;
    public_cfg.binds.get_or_insert_with(Vec::new);
    log::info!("Job {}: got test suite", job.id);

    send.lock()
        .await
        .send_msg(&ClientMsg::JobProgress(JobProgressMsg {
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

    log::info!("Job {}: fetched", job.id);

    let job_path: PathBuf = fs::find_judge_root(&job_path).await?;
    let mut judge_cfg = job_path.clone();
    judge_cfg.push(JUDGE_FILE_NAME);

    log::info!(
        "Job {}: found job description file at {:?}",
        job.id,
        &judge_cfg
    );

    let judge_cfg = tokio::fs::read(judge_cfg).await?;
    let judge_cfg = toml::from_slice::<JudgeToml>(&judge_cfg)?;

    log::info!("Job {}: read job description file", job.id);

    let judge_job_cfg = judge_cfg
        .jobs
        .get(&public_cfg.name)
        .ok_or_else(|| JobExecErr::NoSuchConfig(public_cfg.name.to_owned()))?;

    let image = judge_job_cfg.image.clone();

    log::info!("Job {}: prepare to run", job.id);

    send.lock()
        .await
        .send_msg(&ClientMsg::JobProgress(JobProgressMsg {
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

    let mut tests_path = job_path.clone();
    tests_path.push(&public_cfg.mapped_dir.from);
    let private_cfg = JudgerPrivateConfig {
        test_root_dir: tests_path,
        mapped_test_root_dir: public_cfg.mapped_dir.to.clone(),
    };

    let options = crate::tester::exec::TestSuiteOptions {
        tests: job.tests.clone(),
        time_limit: None,
        mem_limit: None,
        build_image: true,
        remove_image: true,
    };

    let mut suite = crate::tester::exec::TestSuite::from_config(
        image,
        &job_path,
        private_cfg,
        public_cfg,
        options,
    )?;

    log::info!("Job {}: options created", job.id);
    let (ch_send, ch_recv) = tokio::sync::mpsc::unbounded_channel();

    let recv_handle = tokio::spawn({
        let mut recv = ch_recv;
        let ws_send = send.clone();
        let job_id = job.id;
        async move {
            while let Some((key, res)) = recv.next().await {
                log::info!("Job {}: recv message for key={}", job_id, key);
                // Omit error; it doesn't matter
                let _ = ws_send
                    .lock()
                    .await
                    .send_msg(&ClientMsg::PartialResult(PartialResultMsg {
                        job_id,
                        test_id: key,
                        test_result: res,
                    }))
                    .await;
            }
        }
    });

    let docker = bollard::Docker::connect_with_local_defaults().unwrap();

    log::info!("Job {}: started.", job.id);

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
            Some(ch_send),
            Some(upload_info),
            cancel.clone(),
        )
        .await?;

    log::info!("Job {}: finished running", job.id);

    let _ = recv_handle.await;

    log::info!("Job {}: finished", job.id);

    let job_result = JobResultMsg {
        job_id: job.id,
        results: result,
        job_result: JobResultKind::Accepted,
        message: None,
    };
    Ok(job_result)
}

pub async fn flag_new_job(send: Arc<Mutex<WsSink>>, client_config: Arc<SharedClientData>) {
    let job_count = client_config.new_job();
    let _ = send
        .lock()
        .await
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: job_count as i32,
            request_for_new_task: 0,
            can_accept_new_task: job_count < client_config.cfg.max_concurrent_tasks,
        }))
        .await;
}

pub async fn flag_finished_job(send: Arc<Mutex<WsSink>>, client_config: Arc<SharedClientData>) {
    let job_count = client_config.finish_job();
    let _ = send
        .lock()
        .await
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: job_count as i32,
            request_for_new_task: 1,
            can_accept_new_task: job_count < client_config.cfg.max_concurrent_tasks,
        }))
        .await;
}

pub async fn accept_job(
    job: NewJob,
    send: Arc<Mutex<WsSink>>,
    client_config: Arc<SharedClientData>,
) {
    log::info!("Received job {}", job.job.id);
    let job_id = job.job.id;
    let cancel_handle = client_config.cancel_handle.create_child();
    let cancel_token = cancel_handle.get_token();
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
    let job = client_config
        .running_job_handles
        .lock()
        .await
        .remove(&job_id);
    if let Some((handle, cancel)) = job {
        cancel.cancel();
        match handle.await {
            Ok(_) => log::info!("Cancelled job {}", job_id),
            Err(e) => log::warn!("Unable to cancel job {}: {}", job_id, e),
        };
    }
    // remove self from cancelling job list
    client_config
        .cancelling_job_handles
        .lock()
        .await
        .remove(&job_id);
}

pub async fn client_loop(
    mut ws_recv: WsStream,
    mut ws_send: WsSink,
    client_config: Arc<SharedClientData>,
) {
    // Request for max_concurrent_tasks jobs
    ws_send
        .send_msg(&ClientMsg::ClientStatus(ClientStatusMsg {
            active_task_count: 0,
            request_for_new_task: client_config.cfg.max_concurrent_tasks as u32,
            can_accept_new_task: true,
        }))
        .await
        .unwrap();

    let ws_send = Arc::new(Mutex::new(ws_send));
    while let Some(Some(Ok(x))) = ws_recv
        .next()
        .with_cancel(client_config.cancel_handle.get_token())
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
                    log::warn!(
                        "Unable to deserialize mesage: {}\nError: {:?}",
                        String::from_utf8_lossy(&payload),
                        e
                    );
                }
            }
        } else if x.is_ping() {
            // Noop.
        } else {
            log::warn!("Unsupported message: {:?}", x);
        }
    }

    ws_send.lock().await.close().await.unwrap();
    log::warn!("Disconnected!");
}
