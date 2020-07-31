use clap::Clap;
use tokio::{prelude::*, sync::Mutex};
mod opt;
use rurikawa_judger::client::{client_loop, connect_to_coordinator, ConnectConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let opt = opt::Opts::parse();
    match opt.cmd {
        opt::SubCmd::Connect(cmd) => client(cmd).await,
        opt::SubCmd::Run(_) => {}
    }
    println!("Hello world");
}

async fn client(cmd: opt::ConnectSubCmd) {
    let cfg = ConnectConfig {
        host: cmd.host,
        token: cmd.token,
    };
    let conn = connect_to_coordinator(&cfg)
        .await
        .expect("Failed to connect");
    let conn = Arc::new(Mutex::new(conn));
    client_loop(conn.clone()).await;
}
