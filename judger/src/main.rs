use clap::Clap;
use dirs::home_dir;
use rurikawa_judger::client::{
    client_loop, connect_to_coordinator, ClientConfig, SharedClientData,
};
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    let cfg = SharedClientData::new(ClientConfig {
        cache_folder: cmd.temp_folder_path.unwrap_or_else(|| {
            let mut dir =
                home_dir().expect("Failed to get home directory. Please provide a storage folder manually via `--temp-folder-path <path>`");
            dir.push(".rurikawa");
            dir
        }),
        ssl: cmd.ssl,
        host: cmd.host,
        max_concurrent_tasks:4,
        access_token: cmd.access_token,
        register_token: cmd.register_token,
    });
    let client_config = Arc::new(cfg);
    loop {
        let (sink, stream) = connect_to_coordinator(&client_config)
            .await
            .expect("Failed to connect");
        client_loop(stream, sink, client_config.clone()).await;
    }
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
