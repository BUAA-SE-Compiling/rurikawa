pub mod model;

use crate::{fs, prelude::FlowSnake};
use futures::{
    stream::{SplitSink, SplitStream},
    FutureExt, StreamExt,
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
pub struct ConnectConfig {
    pub base: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub host: ConnectConfig,
    pub cache_folder: PathBuf,
}

impl ClientConfig {
    pub fn websocket_endpoint(&self) -> String {
        format!("{}/api/v1/judger/ws", self.host.base)
    }

    pub fn test_suite_download_endpoint(&self, suite_id: FlowSnake) -> String {
        format!("{}/api/v1/test_suite/{}", self.host.base, suite_id)
    }

    pub fn job_folder_root(&self) -> PathBuf {
        let mut job_temp_folder = self.cache_folder.clone();
        job_temp_folder.push("jobs");
        job_temp_folder
    }

    pub fn test_suite_folder_root(&self) -> PathBuf {
        let mut test_suite_temp_folder = self.cache_folder.clone();
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
        let mut test_suite_temp_folder = self.cache_folder.clone();
        test_suite_temp_folder.push("files");
        test_suite_temp_folder
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
    cfg: &ConnectConfig,
) -> Result<(WsSink, WsStream), tungstenite::Error> {
    let mut req = http::Request::builder().uri(&cfg.base);
    if let Some(token) = cfg.token.as_ref() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    log::info!("Connecting to {}", cfg.base);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    log::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

pub struct ActiveJob {}

pub async fn check_and_download_test_suite(
    test_suite: FlowSnake,
    cfg: &ClientConfig,
) -> Result<(), ClientErr> {
    let endpoint = cfg.test_suite_download_endpoint(test_suite);

    tokio::fs::create_dir_all(cfg.test_suite_folder_root()).await?;

    let suite_folder = cfg.test_suite_folder(test_suite);
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

    Ok(())
}

pub async fn handle_job_wrapper(job: NewJob, send: Arc<Mutex<WsSink>>, cfg: Arc<ClientConfig>) {
    // TODO: Handle failed cases and report
    match handle_job(job, send, cfg).await {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub async fn handle_job(
    job: NewJob,
    send: Arc<Mutex<WsSink>>,
    cfg: Arc<ClientConfig>,
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

pub async fn client_loop(mut ws_recv: WsStream, ws_send: WsSink, client_config: Arc<ClientConfig>) {
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
    log::warn!("Disconnected!");
}
