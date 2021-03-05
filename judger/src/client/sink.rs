//! Data structures for handling websocket message sinks that preserve message when connection is not available.

use anyhow::Result;
use arc_swap::{ArcSwapAny, ArcSwapOption};
use async_trait::async_trait;
use futures::{
    stream::{SplitSink, SplitStream},
    Sink, SinkExt, Stream, TryStream,
};
use serde::Serialize;
use std::{fmt::Debug, sync::Arc};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::Message;

use crate::prelude::CancellationTokenHandle;

pub type WsDuplex = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = WebsocketSink;
pub type RawWsSink = SplitSink<WsDuplex, Message>;
pub type WsStream = SplitStream<WsDuplex>;

pub struct WebsocketSink {
    sink: ArcSwapOption<Mutex<RawWsSink>>,
    handle: ArcSwapAny<Arc<CancellationTokenHandle>>,
}

impl WebsocketSink {
    pub fn new() -> WebsocketSink {
        WebsocketSink {
            sink: arc_swap::ArcSwapOption::new(None),
            handle: ArcSwapAny::new(Arc::new(CancellationTokenHandle::new())),
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), tungstenite::Error> {
        self.send_conf(msg, false).await
    }

    pub async fn send_conf(
        &self,
        msg: Message,
        err_if_connection_fail: bool,
    ) -> Result<(), tungstenite::Error> {
        let mut sink = self.sink.load();
        if sink.is_none() && err_if_connection_fail {
            return Err(tungstenite::Error::AlreadyClosed);
        }
        while sink.is_none() {
            // drop guard to avoid deadlock
            drop(sink);
            // wait for sink being connected
            let handle = self.handle.load().child_token();
            handle.cancelled().await;
            sink = self.sink.load();
        }
        sink.clone().unwrap().lock().await.send(msg).await
    }

    pub async fn send_all<It>(&self, msg: &mut It) -> Result<(), tungstenite::Error>
    where
        It: TryStream<Ok = Message, Error = tungstenite::Error>
            + Stream<Item = Result<Message, tungstenite::Error>>
            + Unpin,
    {
        self.send_all_conf(msg, false).await
    }

    pub async fn send_all_conf<It>(
        &self,
        msg: &mut It,
        err_if_connection_fail: bool,
    ) -> Result<(), tungstenite::Error>
    where
        It: TryStream<Ok = Message, Error = tungstenite::Error>
            + Stream<Item = Result<Message, tungstenite::Error>>
            + Unpin,
    {
        let mut sink = self.sink.load();
        if sink.is_none() && err_if_connection_fail {
            return Err(tungstenite::Error::AlreadyClosed);
        }
        while sink.is_none() {
            // drop guard to avoid deadlock
            drop(sink);
            // wait for sink being connected
            let handle = self.handle.load().child_token();
            handle.cancelled().await;
            sink = self.sink.load();
        }
        let sink = sink.clone().unwrap();
        let mut sink = sink.lock().await;
        let inner = &mut *sink;
        inner.send_all(msg).await?;
        Ok(())
    }

    pub fn load_socket(&self, sink: RawWsSink) {
        self.sink.swap(Some(Arc::new(Mutex::new(sink))));
        self.handle
            .swap(Arc::new(CancellationTokenHandle::new()))
            .cancel();
    }

    pub fn clear_socket(&self) {
        self.sink.swap(None);
    }

    pub async fn send_msg<M: Serialize + Sync>(&self, msg: &M) -> Result<(), tungstenite::Error> {
        let serialized = serde_json::to_string(msg).unwrap();
        let msg = Message::text(serialized);
        self.send(msg).await
    }
}

impl Default for WebsocketSink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait SendJsonMessage<M>
where
    M: Serialize,
{
    type Error;
    async fn send_msg(&mut self, msg: &M) -> Result<(), Self::Error>;
}

#[async_trait]
impl<M, T> SendJsonMessage<M> for T
where
    T: Sink<Message> + Unpin + Send + Sync,
    M: Serialize + Sync + Debug,
{
    type Error = T::Error;
    async fn send_msg(&mut self, msg: &M) -> Result<(), Self::Error> {
        // tracing::info!("sent: {:?}", msg);
        let serialized = serde_json::to_string(msg).unwrap();
        let msg = Message::text(serialized);
        self.send(msg).await
    }
}

// #[async_trait]
// impl<M> SendJsonMessage<M> for WebsocketSink
// where
//     M: Serialize + Sync + Debug,
// {
//     type Error = tungstenite::Error;

// }
