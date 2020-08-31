use broadcaster::BroadcastChannel;
use clap::Clap;
use futures::{Future, FutureExt, Sink, SinkExt, StreamExt};
use once_cell::sync::Lazy;
use rurikawa_judger::client::{client_loop, connect_to_coordinator, ClientConfig, ConnectConfig};
use std::{
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{prelude::*, sync::Mutex};
use tungstenite::Message;

mod opt;

static CTRL_C: AtomicBool = AtomicBool::new(false);
static CTRL_C_TWICE: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() {
    let opt = opt::Opts::parse();
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] [{}] {}",
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .expect("Failed to set up logger");

    ctrlc::set_handler(handle_ctrl_c).expect("Failed to set termination handler!");

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
    let (mut sink, mut stream) = connect_to_coordinator(&cfg)
        .await
        .expect("Failed to connect");
    sink.send(Message::text("test!")).await.unwrap();
    println!("{:?}", stream.next().await.unwrap());
    let cfg = ClientConfig {
        temp_folder: "/tmp/".into(),
    };
    client_loop(stream, sink, Arc::new(cfg)).await;
}

fn handle_ctrl_c() {
    if !CTRL_C.load(Ordering::SeqCst) {
        log::warn!("Waiting for existing jobs to complete... Press Ctrl-C again to force quit.");
        CTRL_C.store(true, Ordering::SeqCst);
    } else if !CTRL_C_TWICE.load(Ordering::SeqCst) {
        log::error!("Force quit!");
        CTRL_C.store(true, Ordering::SeqCst);
        CTRL_C_TWICE.store(true, Ordering::SeqCst);
        exit(101);
    } else {
    }
}
