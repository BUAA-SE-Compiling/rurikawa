use crate::prelude::FlowSnake;
use futures::{
    future::FusedFuture,
    stream::{SplitSink, SplitStream},
    Future, FutureExt, Sink, SinkExt, Stream, StreamExt,
};
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::{collections::HashMap, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

pub type WsDuplex = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsDuplex, Message>;
pub type WsStream = SplitStream<WsDuplex>;

/// Message sent from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ServerMsg {
    NewJob(NewJob),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub id: FlowSnake,
    pub pkg_uri: String,
}

/// Message sent from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ClientMsg {
    #[serde(rename = "job_progress")]
    JobProgress(JobProgressMsg),

    #[serde(rename = "job_result")]
    JobResult(JobResultMsg),

    #[serde(rename = "client_status")]
    ClientStatus(ClientStatusMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub kind: TestResultKind,
    pub result_file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressMsg {
    pub id: FlowSnake,
    pub job_stage: JobStage,
    pub total_points: Option<u64>,
    pub finished_points: Option<u64>,
    pub partial_results: HashMap<String, TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultMsg {
    pub job_id: FlowSnake,
    pub results: HashMap<String, TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusMsg {
    pub active_task_count: i32,
    pub can_accept_new_task: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectConfig {
    pub host: String,
    pub token: Option<String>,
}

pub struct ActiveJob {}

pub struct Client {
    running_tasks: HashMap<String, Arc<Mutex<ActiveJob>>>,
}

pub async fn connect_to_coordinator(
    cfg: &ConnectConfig,
) -> Result<(WsSink, WsStream), tungstenite::Error> {
    let mut req = http::Request::builder().uri(&cfg.host);
    if let Some(token) = cfg.token.as_ref() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    log::info!("Connecting to {}", cfg.host);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    log::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

pub async fn client_loop<F, Fut>(mut ws: WsStream, mut test_abort: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()> + FusedFuture + Unpin,
{
    while let Some(Some(Ok(x))) = {
        let mut ws_lock = ws.next().fuse();
        let mut abort_lock = test_abort();
        futures::select_biased! {
            abort = abort_lock => None,
            ws = ws_lock => Some(ws)
        }
    } {
        if x.is_text() {
            let msg = from_slice::<ServerMsg>(&x.into_data());
            if let Ok(msg) = msg {
                log::warn!("TODO: Do stuff with {:?}", msg);
            }
        }
    }
    log::warn!("Disconnected!");
}
