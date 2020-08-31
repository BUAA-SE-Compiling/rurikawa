pub mod model;

use crate::fs;
use futures::{
    stream::{SplitSink, SplitStream},
    FutureExt, StreamExt,
};
use model::*;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

pub type WsDuplex = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsDuplex, Message>;
pub type WsStream = SplitStream<WsDuplex>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectConfig {
    pub host: String,
    pub token: Option<String>,
}

pub struct ClientConfig {
    pub temp_folder: PathBuf,
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

pub struct ActiveJob {}

pub async fn handle_job(job: NewJob, send: Arc<Mutex<WsSink>>, cfg: Arc<ClientConfig>) {
    let job = job.job;
    let mut path = cfg.temp_folder.clone();
    path.push("job");
    path.push(job.id.to_string());

    // Clone the repo specified in job
    fs::net::git_clone(
        Some(&path),
        fs::net::GitCloneOptions {
            repo: job.repo,
            branch: job.branch,
            depth: 3,
        },
    )
    .await
    .unwrap();
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
                        tokio::spawn(handle_job(job, send, client_config.clone()));
                    }
                }
            } else {
                log::warn!("Unknown binary message");
            }
        }
    }
    log::warn!("Disconnected!");
}
