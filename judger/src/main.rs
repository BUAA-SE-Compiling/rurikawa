use clap::Clap;
use dirs::home_dir;
use once_cell::sync::OnceCell;
use rurikawa_judger::{
    client::{
        client_loop, connect_to_coordinator, try_register, verify_self, ClientConfig,
        SharedClientData,
    },
    prelude::CancellationTokenHandle,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{path::Path, process::exit};

mod opt;

static CTRL_C: AtomicBool = AtomicBool::new(false);
static CTRL_C_TWICE: AtomicBool = AtomicBool::new(false);
static ABORT_HANDLE: OnceCell<CancellationTokenHandle> = OnceCell::new();

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
}

async fn read_client_config(source_path: &Path) -> std::io::Result<Option<ClientConfig>> {
    let mut config_path = source_path.to_owned();
    config_path.push("config.toml");

    let res = tokio::fs::read_to_string(&config_path).await;
    let cfg = match res {
        Ok(cfg) => cfg,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => return Ok(None),
            _ => return Err(e),
        },
    };

    let cfg = toml::from_str::<ClientConfig>(&cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(cfg))
}

async fn update_client_config(source_path: &Path, cfg: &ClientConfig) -> std::io::Result<()> {
    let mut config_path = source_path.to_owned();
    config_path.push("config.toml");

    let cfg_str = toml::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    tokio::fs::write(&config_path, cfg_str).await
}

fn override_config_using_cmd(cmd: &opt::ConnectSubCmd, cfg: &mut ClientConfig) {
    if let Some(token) = cmd.access_token.clone() {
        cfg.access_token = Some(token);
    }
    if let Some(token) = cmd.register_token.clone() {
        cfg.register_token = Some(token);
    }
    if let Some(cnt) = cmd.concurrent_tasks {
        cfg.max_concurrent_tasks = cnt;
    }
    if let Some(ssl) = cmd.ssl {
        cfg.ssl = ssl;
    }
    if let Some(host) = cmd.host.clone() {
        cfg.host = host;
    }
    if let Some(tags) = cmd.tag.clone() {
        cfg.tags = Some(tags);
    }
}

async fn client(cmd: opt::ConnectSubCmd) {
    let cache_folder = cmd.temp_folder_path.clone().unwrap_or_else(|| {
            let mut dir =
                home_dir().expect("Failed to get home directory. Please provide a storage folder manually via `--temp-folder-path <path>`");
            dir.push(".rurikawa");
            dir
        });

    let mut cfg = read_client_config(&cache_folder)
        .await
        .unwrap()
        .unwrap_or_default();

    override_config_using_cmd(&cmd, &mut cfg);
    cfg.cache_folder = cache_folder.clone();

    let mut cfg = SharedClientData::new(cfg);

    let verify_res = verify_self(&cfg)
        .await
        .expect("Error when verifying judger status");

    // retry register if not verified or force refresh
    let refresh = !verify_res || cmd.refresh;
    if refresh {
        log::warn!("Verification failed. Registering.");
        let register_res = try_register(&mut cfg, refresh)
            .await
            .expect("Error when registering judger");
        if !register_res {
            panic!("Judger cannot be registered. Please check your register token.");
        }
        if !verify_self(&cfg)
            .await
            .expect("Error when verifying judger status")
        {
            panic!("Judger cannot be verified with the latest access token! This might be a server issue.");
        }
    }

    if !cmd.no_save {
        update_client_config(&cache_folder, &cfg.cfg).await.unwrap();
    }

    let client_config = Arc::new(cfg);
    let handle = client_config.cancel_handle.clone();
    ABORT_HANDLE.set(handle).unwrap();

    loop {
        let (sink, stream) = connect_to_coordinator(&client_config)
            .await
            .expect("Failed to connect");
        client_loop(stream, sink, client_config.clone()).await;
        if client_config.cancel_handle.is_cancelled() {
            break;
        }
    }

    let mut cancelling_guard = client_config.cancelling_job_handles.lock().await;
    let mut cancelling = cancelling_guard.drain().collect::<Vec<_>>();
    let mut running_guard = client_config.running_job_handles.lock().await;
    let mut running = running_guard.drain().collect::<Vec<_>>();
    drop(cancelling_guard);
    drop(running_guard);

    let cancelling = cancelling.drain(..).map(|(id, fut)| {
        log::info!("Waiting for job {} to cancel...", id);
        fut
    });
    let running = running.drain(..).map(|(id, fut)| {
        log::info!("Waiting for job {} to abort...", id);
        fut.0
    });
    futures::future::join_all(cancelling.chain(running)).await;
}

fn handle_ctrl_c() {
    if !CTRL_C.load(Ordering::SeqCst) {
        log::warn!("Waiting for existing jobs to complete... Press Ctrl-C again to force quit.");
        CTRL_C.store(true, Ordering::SeqCst);
        if let Some(x) = ABORT_HANDLE.get() {
            x.cancel();
        }
    } else if !CTRL_C_TWICE.load(Ordering::SeqCst) {
        log::error!("Force quit!");
        CTRL_C.store(true, Ordering::SeqCst);
        CTRL_C_TWICE.store(true, Ordering::SeqCst);
        exit(101);
    } else {
    }
}
