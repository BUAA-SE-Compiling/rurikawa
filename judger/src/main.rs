use clap::Clap;
use rurikawa_judger::client::{client_loop, connect_to_coordinator, ConnectConfig};
use std::{
    process::exit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{prelude::*, sync::Mutex};

mod opt;

static CTRL_C: AtomicBool = AtomicBool::new(false);
static CTRL_C_TWICE: AtomicBool = AtomicBool::new(false);

#[tokio::main]
async fn main() {
    let opt = opt::Opts::parse();
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
    let conn = connect_to_coordinator(&cfg)
        .await
        .expect("Failed to connect");
    let conn = Arc::new(Mutex::new(conn));
    client_loop(conn.clone()).await;
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
