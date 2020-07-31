use tokio::stream::{Stream, StreamExt};
use tokio_tungstenite::connect_async;

pub struct ConnectConfig {
    host: String,
}

pub async fn connect_to_coordinator(cfg: ConnectConfig) {
    let (client, _) = connect_async(&cfg.host)
        .await
        .expect("Unable to connect to server");
}
