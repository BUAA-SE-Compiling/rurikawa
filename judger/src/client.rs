use futures::{Sink, SinkExt};
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use std::sync::Arc;
use tokio::{
    net::TcpStream,
    stream::{Stream, StreamExt},
    sync::Mutex,
};
use tokio_tungstenite::{connect_async, tungstenite, WebSocketStream};

/// Message sent from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ServerMsg {
    NewJob(NewJob),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub id: String,
    pub pkg_uri: String,
}

/// Message sent from client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_t")]
pub enum ClientMsg {
    JobProcess,
    JobResult,
}

pub struct ConnectConfig {
    pub host: String,
    pub token: Option<String>,
}

pub async fn connect_to_coordinator(
    cfg: &ConnectConfig,
) -> Result<WebSocketStream<TcpStream>, tungstenite::Error> {
    let mut req = http::Request::builder().uri(&cfg.host);
    if let Some(token) = cfg.token.as_ref() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    log::info!("Connecting to {}", cfg.host);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    log::info!("Connection success");
    Ok(client)
}

pub async fn client_loop(ws: Arc<Mutex<WebSocketStream<TcpStream>>>) {
    while let Some(Ok(x)) = ws.lock().await.next().await {
        if x.is_text() {
            let msg = from_slice::<ServerMsg>(&x.into_data());
            if let Ok(msg) = msg {
                log::warn!("TODO: Do stuff with {:?}", msg);
            }
        }
    }
    log::warn!("Disconnected!");
}
