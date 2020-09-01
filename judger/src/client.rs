pub mod model;

use crate::{fs, prelude::*};
use dashmap::DashMap;
use futures::{
    stream::{SplitSink, SplitStream},
    FutureExt, SinkExt, StreamExt,
};
use http::Uri;
use model::*;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

pub type WsDuplex = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsDuplex, Message>;
pub type WsStream = SplitStream<WsDuplex>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub host: String,
    pub token: Option<String>,
    pub cache_folder: PathBuf,
}

#[derive(Debug)]
pub struct SharedClientData {
    pub cfg: ClientConfig,
    /// All test suites whose folder is being edited.
    ///
    ///
    pub locked_test_suite: DashMap<FlowSnake, Arc<Mutex<()>>>,
}

impl SharedClientData {
    pub fn new(cfg: ClientConfig) -> SharedClientData {
        SharedClientData {
            cfg,
            locked_test_suite: DashMap::new(),
        }
    }

    pub fn websocket_endpoint(&self) -> String {
        format!("{}/api/v1/judger/ws", self.cfg.host)
    }

    pub fn test_suite_download_endpoint(&self, suite_id: FlowSnake) -> String {
        format!("{}/api/v1/test_suite/{}", self.cfg.host, suite_id)
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

    pub async fn obtain_suite_lock<'a>(&self, suite_id: FlowSnake) -> Arc<Mutex<()>> {
        let cur = self
            .locked_test_suite
            .get(&suite_id)
            .map(|pair| pair.value().clone());
        if let Some(cur) = cur {
            cur
        } else {
            let arc = Arc::new(Mutex::new(()));
            self.locked_test_suite.insert(suite_id, arc.clone());
            arc
        }
    }
}

#[derive(Debug)]
pub enum ClientErr {
    Io(std::io::Error),
    Boxed(Box<dyn Error>),
}

impl From<std::io::Error> for ClientErr {
    fn from(e: std::io::Error) -> Self {
        ClientErr::Io(e)
    }
}

impl From<Box<dyn Error>> for ClientErr {
    fn from(e: Box<dyn Error>) -> Self {
        ClientErr::Boxed(e)
    }
}

pub async fn connect_to_coordinator(
    cfg: &SharedClientData,
) -> Result<(WsSink, WsStream), tungstenite::Error> {
    let endpoint = cfg.websocket_endpoint();
    let mut req = http::Request::builder().uri(&endpoint);
    if let Some(token) = cfg.cfg.token.as_ref() {
        req = req.header("Authorization", format!("Bearer {}", token));
    } else {
        req = req.header("Authorization", "");
    }
    log::info!("Connecting to {}", endpoint);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    log::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

pub struct ActiveJob {}

pub async fn check_and_download_test_suite(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<(), ClientErr> {
    let endpoint = cfg.test_suite_download_endpoint(suite_id);

    tokio::fs::create_dir_all(cfg.test_suite_folder_root()).await?;
    {
        // Lock this specific test suite and let all other concurrent tasks to wait
        // until downloading completes
        let lock = cfg.obtain_suite_lock(suite_id).await;
        lock.lock().await;

        let suite_folder = cfg.test_suite_folder(suite_id);
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
            fs::net::download_unzip(&endpoint, &suite_folder, &cfg.temp_file_folder_root()).await?;
        }
    }

    Ok(())
}

pub async fn handle_job_wrapper(job: NewJob, send: Arc<Mutex<WsSink>>, cfg: Arc<SharedClientData>) {
    // TODO: Handle failed cases and report
    match handle_job(job, send, cfg).await {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub async fn handle_job(
    job: NewJob,
    send: Arc<Mutex<WsSink>>,
    cfg: Arc<SharedClientData>,
) -> Result<(), ClientErr> {
    let job = job.job;

    check_and_download_test_suite(job.test_suite, &*cfg).await?;

    let path = cfg.job_folder(job.id);

    // Clone the repo specified in job
    fs::net::git_clone(
        Some(&path),
        fs::net::GitCloneOptions {
            repo: job.repo,
            branch: job.branch,
            depth: 3,
        },
    )
    .await?;

    Ok(())
}

pub async fn client_loop(
    mut ws_recv: WsStream,
    mut ws_send: WsSink,
    client_config: Arc<SharedClientData>,
) {
    let ready_msg = ClientMsg::ClientStatus(ClientStatusMsg {
        active_task_count: 0,
        can_accept_new_task: true,
    });
    let ready_msg_ser = serde_json::to_string(&ready_msg).unwrap();
    log::info!("{}", ready_msg_ser);
    ws_send.send(Message::Text(ready_msg_ser)).await.unwrap();

    let ws_send = Arc::new(Mutex::new(ws_send));
    while let Some(Some(Ok(x))) = {
        let mut ws_lock = ws_recv.next().fuse();
        // TODO: add abort mechaisms
        // let mut abort_lock = abort.next().fuse();
        let mut abort_lock = futures::future::pending::<()>();
        futures::select_biased! {
            abort = abort_lock => None,
            ws = ws_lock => Some(ws)
        }
    } {
        if x.is_text() {
            let msg = from_slice::<ServerMsg>(&x.into_data());
            if let Ok(msg) = msg {
                match msg {
                    ServerMsg::NewJob(job) => {
                        let send = ws_send.clone();
                        tokio::spawn(handle_job_wrapper(job, send, client_config.clone()));
                    }
                }
            } else {
                log::warn!("Unknown binary message");
            }
        }
    }

    ws_send.lock().await.close().await.unwrap();
    log::warn!("Disconnected!");
}
