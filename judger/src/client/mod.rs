pub mod config;
mod err;
pub mod model;
pub mod sink;

pub use self::err::*;
use self::{
    config::{ClientConfig, SharedClientData},
    model::*,
    sink::*,
};
use crate::{
    config::{JudgeToml, JudgerPublicConfig},
    fs::{self, JUDGE_FILE_NAME},
    prelude::*,
    runner::{exec::CreateContainerConfigBuilder, volume::Volume},
    tester::{
        build_judger_container, build_user_code_container, model::Bind,
        runner_plan::RawTestCaseResult,
    },
    util::AsyncTeardownCollector,
};
use anyhow::{Context, Result};
use futures::prelude::*;
use http::Method;
use ignore::gitignore::Gitignore;
use itertools::Itertools;
use respector::prelude::*;
use serde_json::from_slice;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    sync::Arc,
};
use stream::StreamExt;
use tokio::sync::OwnedMutexGuard;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info_span, instrument};
use tracing_futures::Instrument;

/// Try to register at the coordinator if no access token was specified.
///
/// Returns `Ok(true)` if register was success, `Ok(false)` if register is not
/// needed or not applicable.
pub async fn try_register(
    client_data: &mut SharedClientData,
    refresh: bool,
) -> anyhow::Result<bool> {
    tracing::info!(
        "Registering judger. Access token: {:?}; Register token: {:?}",
        client_data.cfg().access_token,
        client_data.cfg().register_token
    );
    if (!refresh && client_data.cfg().access_token.is_some())
        || client_data.cfg().register_token.is_none()
    {
        return Ok(false);
    }

    let req_body = JudgerRegisterMessage {
        token: client_data.cfg().register_token.clone().unwrap(),
        alternate_name: client_data.cfg().alternate_name.clone(),
        tags: client_data.cfg().tags.clone(),
    };
    let endpoint = client_data.register_endpoint();
    let client = &client_data.client;

    let req = client
        .request(Method::POST, &endpoint)
        .json(&req_body)
        .build()?;
    let res = client.execute(req).await?;

    let status = res.status().as_u16();
    if status >= 300 {
        let headers = res.headers();
        tracing::error!("Failed to register judger. Status: {}", status);
        tracing::error!("Headers: {:#?}", headers);
        let body = res.text().await?;
        tracing::error!("body: {}", body);
        return Err(anyhow::Error::msg(format!(
            "Failed to register judger: status code {}",
            status
        )));
    }
    let res = res.text().await?;

    tracing::info!("Got new access token: {}", res);

    let new_cfg = ClientConfig {
        access_token: Some(res),
        ..(**client_data.cfg()).clone()
    };
    client_data.swap_cfg(Arc::new(new_cfg));

    Ok(true)
}

/// Verify if the current registration is active.
pub async fn verify_self(cfg: &SharedClientData) -> anyhow::Result<bool> {
    tracing::info!("Verifying access token {:?}", cfg.cfg().access_token);
    if cfg.cfg().access_token.is_none() {
        return Ok(false);
    }

    let endpoint = cfg.verify_endpoint();
    let res = cfg
        .client
        .request(Method::GET, &endpoint)
        .header("authorization", cfg.cfg().access_token.as_ref().unwrap())
        .send()
        .await?
        .status()
        .is_success();
    Ok(res)
}

pub async fn connect_to_coordinator(
    cfg: &SharedClientData,
) -> Result<(RawWsSink, WsStream), ClientConnectionErr> {
    let endpoint = cfg.websocket_endpoint();
    let req = http::Request::builder().uri(&endpoint);
    tracing::info!("Connecting to {}", endpoint);
    let (client, _) = connect_async(req.body(()).unwrap()).await?;
    let (cli_sink, cli_stream) = client.split();
    tracing::info!("Connection success");
    Ok((cli_sink, cli_stream))
}

async fn fetch_test_suite_data(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<TestSuite, JobExecErr> {
    tracing::info!("Fetching data for test suite {}", suite_id);
    let suite_endpoint = cfg.test_suite_info_endpoint(suite_id);
    let res = cfg
        .client
        .get(&suite_endpoint)
        .send()
        .await?
        .json::<TestSuite>()
        .await?;
    Ok(res)
}

/// Check for updates on this test suite and update if necessary. Reads the test suite
/// and returns.
///
/// Returns the updated public config, its unique ID, and the lock guard for modification
/// to the caller.
///
/// The lock guard is returned in order to prevent modification between suite update
/// and running.
#[instrument(skip(cfg))]
pub async fn check_download_read_test_suite(
    suite_id: FlowSnake,
    cfg: &SharedClientData,
) -> Result<(JudgerPublicConfig, String, OwnedMutexGuard<()>), JobExecErr> {
    let _might_modify_guard = cfg.before_suite_might_modify(suite_id).await;

    tracing::info!("Checking test suite");
    let suite_folder_root = cfg.test_suite_folder_root();
    tokio::fs::create_dir_all(suite_folder_root).await?;
    let suite_folder = cfg.test_suite_folder(suite_id);

    tracing::debug!("Folder created: {:?}", suite_folder);

    // Lock this specific test suite and let all other concurrent tasks to wait
    // until downloading completes

    let suite_data = fetch_test_suite_data(suite_id, cfg).await?;

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

    // check if test suite is up to date
    let lockfile = cfg.test_suite_folder_lockfile(suite_id);

    let lockfile_up_to_date = {
        let lockfile_data = tokio::fs::read_to_string(&lockfile).await;
        let lockfile_data = match lockfile_data {
            Ok(f) => Some(f),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => None,
                _ => return Err(e.into()),
            },
        };

        let suite_data_locked = lockfile_data
            .as_deref()
            .and_then(|x| serde_json::from_str::<TestSuite>(x).ok());

        suite_data_locked
            .map(|locked| locked.package_file_id == suite_data.package_file_id)
            .unwrap_or(false)
    };

    // download the test suite
    if !dir_exists || !lockfile_up_to_date {
        tracing::info!("Test suite not up to date. Updating...");
        let _modify_guard = cfg.before_suite_modify(suite_id).await;

        let endpoint = cfg.test_suite_download_endpoint(suite_id);
        let file_folder_root = cfg.temp_file_folder_root();

        fs::ensure_removed_dir(&suite_folder).await?;
        tokio::fs::create_dir_all(file_folder_root).await?;
        tracing::info!(
            "Test suite does not exist. Initiating download of suite {} from {} to {}",
            suite_id,
            &endpoint,
            &suite_folder.display()
        );
        fs::net::download_unzip(
            cfg.client.clone(),
            cfg.client
                .get(&endpoint)
                .header("authorization", cfg.cfg().access_token.as_ref().unwrap())
                .build()?,
            &suite_folder,
        )
        .await?;
        tracing::info!("Update completed");
    }

    // Rewrite lockfile AFTER all data are saved
    if !lockfile_up_to_date {
        tracing::info!("Rewriting lockfile");
        let serialized = serde_json::to_string(&suite_data)?;
        tokio::fs::write(&lockfile, &serialized).await?;
    }

    // Note:
    // Lockfile is updated only AFTER test suite is fully downloaded, so an incomplete
    // download would not result in an updated lockfile. Therefore there's no need
    // to clean up the suite folder if things blow up - they're simply ignored.
    //
    // This should be easier to write using traditional try-catch-finally pattern
    // since finally-blocks can also be async. Sadly we don't have AsyncDrop trait
    // yet here in Rust. See this withoutboats' post for more information:
    // <https://without.boats/blog/poll-drop/>
    //
    //   |
    //   V
    // let _ = fs::ensure_removed_dir(&cfg.test_suite_folder(suite_id)).await;

    let mut judger_conf_dir = suite_folder.clone();
    judger_conf_dir.push("testconf.json");
    tracing::debug!("Reading test config: {:?}", judger_conf_dir);
    let judger_conf = match tokio::fs::read(&judger_conf_dir).await {
        Ok(c) => c,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                return Err(JobExecErr::NoSuchFile(
                    judger_conf_dir.to_string_lossy().into_owned(),
                ));
            }
            _ => return Err(JobExecErr::Io(e)),
        },
    };
    let judger_conf = serde_json::from_slice::<JudgerPublicConfig>(&judger_conf)?;
    let unique_id = crate::util::names::transform_string_as_docker_tag(&suite_data.package_file_id);

    Ok((judger_conf, unique_id.into_owned(), _might_modify_guard))
}

fn extract_job_err(job_id: FlowSnake, err: &JobExecErr) -> ClientMsg {
    tracing::warn!("job {} aborted because of error: {:?}", job_id, &err);

    let (err, msg) = match err {
        JobExecErr::NoSuchFile(f) => (
            JobResultKind::CompileError,
            format!("Cannot find file: {}", f),
        ),
        JobExecErr::NoSuchConfig(f) => (
            JobResultKind::CompileError,
            format!("Cannot find config for {} in `judge.toml`", f),
        ),
        JobExecErr::Io(e) => (JobResultKind::JudgerError, format!("IO error: {}", e)),
        JobExecErr::Ws(e) => (
            JobResultKind::JudgerError,
            format!("Websocket error: {:?}", e),
        ),
        JobExecErr::Json(e) => (JobResultKind::JudgerError, format!("JSON error: {:?}", e)),
        JobExecErr::TomlDes(e) => (
            JobResultKind::JudgerError,
            format!("TOML deserialization error: {:?}", e),
        ),
        JobExecErr::Request(e) => (
            JobResultKind::JudgerError,
            format!("Web request error: {:?}", e),
        ),
        JobExecErr::Build(e) => (JobResultKind::CompileError, format!("{}", e)),
        JobExecErr::Exec(e) => (JobResultKind::PipelineError, format!("{:?}", e)),
        JobExecErr::Any(e) => {
            let mut real_err = None;
            for e in e.chain() {
                if let Some(err) = e.downcast_ref::<JobExecErr>() {
                    real_err = Some(err);
                } else {
                    tracing::warn!("    ctx: {}", e);
                }
            }
            if let Some(e) = real_err {
                return extract_job_err(job_id, e);
            } else {
                (JobResultKind::OtherError, format!("{:?}", e))
            }
        }
        JobExecErr::Git(e) => (JobResultKind::CompileError, format!("{}", e)),
        JobExecErr::Cancelled | JobExecErr::Aborted => {
            unreachable!()
        }
    };

    ClientMsg::JobResult(JobResultMsg {
        job_id,
        results: HashMap::new(),
        job_result: err,
        message: Some(msg),
    })
}

pub async fn handle_job_wrapper(
    job: Job,
    send: Arc<WsSink>,
    cancel: CancellationTokenHandle,
    cfg: Arc<SharedClientData>,
) {
    let job_id = job.id;
    flag_new_job(send.clone(), cfg.clone()).await;
    let teardown_collector = AsyncTeardownCollector::new();

    let res_handle = handle_job(
        job,
        send.clone(),
        cancel.clone(),
        cfg.clone(),
        &teardown_collector,
    )
    .instrument(tracing::info_span!("handle_job", %job_id))
    .await;

    teardown_collector.teardown_all().await;

    let msg = match res_handle {
        Ok(_res) => ClientMsg::JobResult(_res),
        // These two types need explicit handling, since they are not finished
        Err(JobExecErr::Aborted) => ClientMsg::JobProgress(JobProgressMsg {
            job_id,
            stage: JobStage::Aborted,
        }),
        Err(JobExecErr::Cancelled) => ClientMsg::JobProgress(JobProgressMsg {
            job_id,
            stage: if cfg.abort_handle.is_cancelled() {
                JobStage::Aborted
            } else if cfg
                .cancelling_job_info
                .get(&job_id)
                .map_or(true, |x| x.as_cancel)
            {
                JobStage::Cancelled
            } else {
                JobStage::Aborted
            },
        }),
        Err(e) => extract_job_err(job_id, &e),
    };

    loop {
        // Ah yes, do-while pattern
        let mut req = cfg.client.post(&cfg.result_send_endpoint()).json(&msg);
        if let Some(token) = &cfg.cfg().access_token {
            req = req.header("authorization", token.as_str());
        }
        let res = req
            .send()
            .and_then(|r| async {
                let status = r.status();
                if !status.is_success() {
                    r.text().await.inspect(|t| {
                        tracing::error!("Error when sending job result mesage: {}\n{}", status, t)
                    })?;
                }
                Ok(())
            })
            .await;
        if res.is_ok() {
            break;
        }
    }

    flag_finished_job(cfg.clone()).await;

    tracing::info!("{}: Result message sent", job_id);

    {
        cfg.running_job_handles.lock().await.remove(&job_id);
    }

    let _ = fs::ensure_removed_dir(&cfg.job_folder(job_id))
        .await
        .inspect_err(|e| tracing::error!("Failed to remove directory for job {}: {}", job_id, e));
    tracing::info!("{}: cleanup complete", job_id);
}

pub async fn handle_job(
    job: Job,
    send: Arc<WsSink>,
    cancel: CancellationTokenHandle,
    cfg: Arc<SharedClientData>,
    teardown_collector: &AsyncTeardownCollector,
) -> Result<JobResultMsg, JobExecErr> {
    // TODO: Move this to program start
    let docker =
        bollard::Docker::connect_with_local_defaults().expect("Unable to connect to docker");

    // INFO: See locking pattern in [`crate::client::config::TestSuiteStatus`].
    let _job_guard = cfg.clone().before_job_start(job.test_suite);

    tracing::info!("Starting");

    let (public_cfg, suite_unique_name, might_modify_permit) =
        pull_public_cfg(&job, &cfg, &cancel).await?;

    // NOTE: We must acquire the read guard before dropping the modify guard.
    // See locking pattern in [`crate::client::config::TestSuiteStatus`].
    let _job_read_guard = cfg.on_suite_run(job.test_suite).await;
    drop(might_modify_permit);

    send.send_msg(&ClientMsg::JobProgress(JobProgressMsg {
        job_id: job.id,
        stage: JobStage::Fetching,
    }))
    .await?;

    // Clone the repo specified in job
    let judge_cfg = pull_job(&cfg, &job, cancel.clone()).await?;

    tracing::info!("read job description file");

    let judge_job_cfg = judge_cfg
        .jobs
        .get(&public_cfg.name)
        .ok_or_else(|| JobExecErr::NoSuchConfig(public_cfg.name.to_owned()))
        .context("parsing judger public config")?;

    let image = judge_job_cfg.image.clone();

    // Check job paths to be relative & does not navigate into parent
    if let crate::tester::model::Image::Dockerfile { path, .. } = &image {
        crate::util::path_security::assert_child_path(path)
            .context("testing if config references external path")?;
        // Note: There's no hard links in a git repository, and also we can't
        // detect them. However, soft (symbolic) links are possible and may
        // point to strange places. We make sure that we haven't got any of
        // those in our paths.
        crate::util::path_security::assert_no_symlink_in_path(path)
            .await
            .context("testing if config has no symlink in path")?;
    }

    tracing::info!("Compiling job image & building container");

    send.send_msg(&ClientMsg::JobProgress(JobProgressMsg {
        job_id: job.id,
        stage: JobStage::Compiling,
    }))
    .await?;

    tracing::debug!("Creating data volume");
    let data_volume_name = format!("rurikawa-judge-data-{}", &job.id);
    let data_volume = Arc::new(
        populate_data_volume(
            &docker,
            &public_cfg,
            &cfg.test_suite_folder(job.test_suite),
            data_volume_name,
        )
        .with_cancel(cancel.cancelled())
        .await
        .ok_or(JobExecErr::Cancelled)?
        .context("Error when populating data volume")?,
    );
    teardown_collector.add(data_volume.clone());

    tracing::debug!("Calculating mount metadata");
    let mounts = public_cfg
        .binds
        .iter()
        .map(Bind::to_mount)
        .chain([data_volume.as_mount(&public_cfg.mapped_dir.to, false)])
        .collect_vec();

    let test_suite_container_cfg = CreateContainerConfigBuilder::default()
        .cancellation(cancel.clone())
        .network_enabled(true)
        .tag_name(format!("judger_container_{}", job.id))
        .mounts(mounts.clone())
        .build()
        .expect("Error when initiating suite container");

    tracing::debug!("Creating judger container");
    let judger_container = build_judger_container(
        docker.clone(),
        &public_cfg,
        &cfg.test_suite_folder(job.test_suite),
        &suite_unique_name,
        test_suite_container_cfg,
    )
    .await?
    .map(Arc::new);
    if let Some(c) = judger_container.clone() {
        teardown_collector.add(c)
    }

    tracing::info!("Building container");

    let (build_ch_send, build_ch_recv) =
        tokio::sync::mpsc::unbounded_channel::<bollard::models::BuildInfo>();

    let build_recv_handle = tokio::spawn({
        let mut recv = build_ch_recv;
        let ws_send = send.clone();
        let job_id = job.id;
        async move {
            while let Some(res) = recv.recv().await {
                let _ = ws_send
                    .send_msg(&ClientMsg::JobOutput(JobOutputMsg {
                        job_id,
                        stream: res.stream,
                        error: res.error,
                    }))
                    .await;
            }
        }
    });

    let user_container = build_user_code_container(
        docker,
        &job.id.to_string(),
        &image,
        |opt| {
            opt.base_path(cfg.job_folder(job.id))
                .cancellation(cancel.clone())
                .build_result_channel(build_ch_send)
                .network_enabled(public_cfg.network.enable_build)
        },
        |opt| {
            opt.mounts(mounts)
                .cancellation(cancel.clone())
                .network_enabled(public_cfg.network.enable_running)
                .tag_name(format!("user_code_container_{}", job.id))
        },
    )
    .await?;
    let user_container = Arc::new(user_container);
    teardown_collector.add(user_container.clone());

    send.send_msg(&ClientMsg::JobProgress(JobProgressMsg {
        job_id: job.id,
        stage: JobStage::Running,
    }))
    .await?;

    tracing::info!("started");

    let upload_info = Arc::new(ResultUploadConfig {
        client: cfg.client.clone(),
        endpoint: cfg.result_upload_endpoint(),
        access_token: cfg.cfg().access_token.clone(),
        job_id: job.id,
    });

    // an arbitrary number is selected for this channel. 4 seems to be more than enough btw
    let (ch_send, ch_recv) = tokio::sync::mpsc::channel(4);

    let recv_handle = tokio::spawn({
        let mut recv = ch_recv;
        let ws_send = send.clone();
        let job_id = job.id;
        let upload_info = upload_info.clone();
        async move {
            let mut final_result = HashMap::new();
            while let Some(RawTestCaseResult(test_case, failure, output)) = recv.recv().await {
                tracing::info!(%job_id, %test_case, "Reporting case result");

                let test_result = transform_and_upload_test_result(
                    failure,
                    output,
                    upload_info.clone(),
                    &test_case,
                )
                .await;

                // Omit error; it doesn't matter
                let _ = ws_send
                    .send_msg(&ClientMsg::PartialResult(PartialResultMsg {
                        job_id,
                        test_id: test_case.clone(),
                        test_result: test_result.clone(),
                    }))
                    .await;

                final_result.insert(test_case, test_result);
            }
            final_result
        }
    });

    let sink = tokio_util::sync::PollSender::new(ch_send).sink_map_err(|_e| ());
    let sink = Box::pin(sink);

    crate::tester::runner_plan::run_job_test_cases(
        &job,
        &public_cfg,
        judge_job_cfg,
        user_container,
        judger_container.map(|container| container as _),
        sink,
        &cfg.test_suite_folder(job.test_suite),
        cancel.clone(),
    )
    .await?;

    if cancel.is_cancelled() {
        // the job is cancelled, but `run_job_test_cases` can't handle this
        // issue since it only reports an untyped `anyhow::Error`, thus cannot
        // report concrete cancellation types.
        return Err(JobExecErr::Cancelled);
    }

    tracing::info!("finished running");

    let _ = build_recv_handle.await;
    let result = recv_handle
        .await
        .map_err(|e| anyhow::anyhow!("The result handling task encountered an error").context(e))?;

    tracing::info!("finished");

    let job_result = JobResultMsg {
        job_id: job.id,
        results: result,
        job_result: JobResultKind::Accepted,
        message: None,
    };
    Ok(job_result)
}

async fn populate_data_volume(
    docker: &bollard::Docker,
    public_cfg: &JudgerPublicConfig,
    base_dir: &Path,
    data_volume_name: String,
) -> Result<Volume, JobExecErr> {
    let mut data_volume = Volume::create(docker.clone(), data_volume_name.clone())
        .await
        .map_err(|e| {
            JobExecErr::Build(crate::tester::model::BuildError::Internal(
                anyhow::Error::new(e).context("When creating data volume"),
            ))
        })?;

    if let Err(e) = data_volume
        .copy_local_files_into(
            &base_dir.join(&public_cfg.mapped_dir.from),
            Gitignore::empty(),
        )
        .await
        .context("When copying local files into target volume")
    {
        let _ = data_volume.remove().await;
        return Err(e.into());
    };

    Ok(data_volume)
}

async fn pull_job(
    cfg: &Arc<SharedClientData>,
    job: &Job,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<JudgeToml, JobExecErr> {
    let job_path = cfg.job_folder(job.id);
    let _ = fs::ensure_removed_dir(&job_path).await;
    fs::net::git_clone(
        &job_path,
        fs::net::GitCloneOptions {
            repo: job.repo.clone(),
            revision: job.revision.clone(),
            depth: 3,
        },
    )
    .with_cancel(cancel.cancelled())
    .await
    .ok_or(JobExecErr::Aborted)?
    .map_err(JobExecErr::Git)
    .context("cloning repo")?;

    tracing::info!("fetched");

    let job_path: PathBuf = fs::find_judge_root(&job_path)
        .await
        .context("finding judger root")?;

    let mut judge_cfg = job_path.clone();

    judge_cfg.push(JUDGE_FILE_NAME);

    tracing::info!("found job description file at {:?}", &judge_cfg);

    let judge_cfg = tokio::fs::read(judge_cfg)
        .await
        .context("reading config file")?;

    let judge_cfg = toml::from_slice::<JudgeToml>(&judge_cfg).context("parsing judger config")?;

    Ok(judge_cfg)
}

async fn pull_public_cfg(
    job: &Job,
    cfg: &Arc<SharedClientData>,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<(JudgerPublicConfig, String, OwnedMutexGuard<()>), JobExecErr> {
    tracing::info!("Checking updates on test suite config");
    let res = check_download_read_test_suite(job.test_suite, &**cfg)
        .with_cancel(cancel.cancelled())
        .instrument(info_span!("download_test_suites", %job.test_suite))
        .await
        .ok_or(JobExecErr::Cancelled)?
        .context("fetching public config")?;
    Ok(res)
}

pub async fn flag_new_job(_send: Arc<WsSink>, client_config: Arc<SharedClientData>) {
    client_config.new_job();
}

pub async fn flag_finished_job(client_config: Arc<SharedClientData>) {
    client_config.finish_job();
}

pub async fn accept_job(job: Job, send: Arc<WsSink>, client_config: Arc<SharedClientData>) {
    tracing::info!("Received job {}", job.id);
    let job_id = job.id;
    let cancel_handle = client_config.abort_handle.child_token();
    let cancel_token = cancel_handle.child_token();

    // Cancel job after timeout
    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            // Hardcoded 30mins.
            // TODO: change this
            tokio::time::sleep(std::time::Duration::from_secs(30 * 60)).await;
            cancel_token.cancel();
        }
    });

    let handle = tokio::spawn(handle_job_wrapper(
        job,
        send,
        cancel_token,
        client_config.clone(),
    ));
    client_config
        .running_job_handles
        .lock()
        .await
        .insert(job_id, (handle, cancel_handle));
}

async fn cancel_job(
    job: AbortJob,
    client_config: Arc<SharedClientData>,
    inserted: futures::channel::oneshot::Receiver<()>,
) {
    let job_id = job.job_id;
    client_config.cancelling_job_info.insert(job_id, job);
    let job = client_config
        .running_job_handles
        .lock()
        .await
        .remove(&job_id);

    if let Some((handle, cancel)) = job {
        cancel.cancel();
        match handle.await {
            Ok(_) => tracing::info!("Cancelled job {}", job_id),
            Err(e) => tracing::warn!("Unable to cancel job {}: {}", job_id, e),
        };
    }

    // Wait until self gets inserted into cancelling job handles
    // (it's a racing condition with the main loop)
    if inserted.await.is_ok() {
        // remove self from cancelling job list
        client_config
            .cancelling_job_handles
            .lock()
            .await
            .remove(&job_id);
    }
    client_config.cancelling_job_info.remove(&job_id);
}

async fn keepalive(
    client_config: Arc<SharedClientData>,
    keepalive_token: CancellationTokenHandle,
    ws: Arc<WsSink>,
    interval: std::time::Duration,
) {
    while tokio::time::sleep(interval)
        .with_cancel(client_config.abort_handle.cancelled())
        .await
        .is_some()
    {
        if let Err(e) = ws
            .send_conf(tokio_tungstenite::tungstenite::Message::Ping(vec![]), true)
            .await
        {
            keepalive_token.cancel();
            tracing::error!("Server disconnected: {}", e);
            break;
        };
    }
}

async fn poll_jobs(
    client_config: Arc<SharedClientData>,
    keepalive_token: CancellationTokenHandle,
    ws: Arc<WsSink>,
    poll_interval: std::time::Duration,
    retry_interval: std::time::Duration,
    poll_timeout: std::time::Duration,
) {
    'outer: loop {
        while client_config.waiting_for_jobs.load().is_some() {
            tracing::debug!("Loading current poll but it's Some(_)...");
            if tokio::time::sleep(retry_interval)
                .with_cancel(keepalive_token.cancelled())
                .await
                .is_none()
            {
                break 'outer;
            }
        }

        let message_id = FlowSnake::generate();

        client_config
            .waiting_for_jobs
            .store(Some(Arc::new(message_id)));

        let active_task_count = client_config.running_tests.load(Ordering::SeqCst) as u32;
        let request_for_new_task =
            client_config.cfg().max_concurrent_tasks as u32 - active_task_count;

        tracing::debug!(
            "Polling jobs from server. Asking for {} new jobs.",
            request_for_new_task
        );

        let msg = ClientMsg::JobRequest(JobRequestMsg {
            active_task_count,
            request_for_new_task,
            message_id: Some(message_id),
        });
        if let Some(Ok(_)) = ws
            .send_msg(&msg)
            .with_cancel(keepalive_token.cancelled())
            .await
        {
            // wtf here
        } else {
            break 'outer;
        }

        tokio::spawn({
            // auto-resetting
            let client_config = client_config.clone();
            async move {
                tokio::time::sleep(poll_timeout).await;
                let old_val = client_config.waiting_for_jobs.swap(None);
                if old_val.as_ref().map_or(false, |x| **x == message_id) {
                    tracing::warn!(
                        "Job polling timed out at {}s for poll message {}. Please check server!",
                        poll_timeout.as_secs_f32(),
                        old_val.unwrap()
                    );
                }
            }
        });

        if tokio::time::sleep(poll_interval)
            .with_cancel(keepalive_token.cancelled())
            .await
            .is_none()
        {
            break 'outer;
        }
    }
    tracing::info!("Stopping current polling session");
}

#[allow(clippy::if_same_then_else)]
pub async fn client_loop(
    mut ws_recv: WsStream,
    ws_send: Arc<WsSink>,
    client_config: Arc<SharedClientData>,
) -> Arc<WsSink> {
    let keepalive_token = client_config.abort_handle.child_token();
    let keepalive_cancel = keepalive_token.child_token();

    client_config.waiting_for_jobs.store(None);

    let keepalive_handle = tokio::spawn(keepalive(
        client_config.clone(),
        keepalive_token,
        ws_send.clone(),
        std::time::Duration::from_secs(20),
    ));

    let poll_jobs_handle = tokio::spawn(poll_jobs(
        client_config.clone(),
        keepalive_cancel.child_token(),
        ws_send.clone(),
        std::time::Duration::from_secs(10),
        std::time::Duration::from_secs(1),
        std::time::Duration::from_secs(60),
    ));

    while let Some(Ok(x)) = ws_recv
        .next()
        .with_cancel(keepalive_cancel.cancelled())
        .await
        .flatten()
    {
        match x {
            Message::Text(payload) => {
                let msg = from_slice::<ServerMsg>(payload.as_bytes());
                if let Ok(msg) = msg.inspect_err(|e| {
                    tracing::warn!("Unable to deserialize mesage: {}\nError: {:?}", &payload, e);
                }) {
                    tracing::debug!("Received message: {:?}", msg);
                    match msg {
                        ServerMsg::MultiNewJob(msg) => {
                            let mut proceed = true;
                            if let Some(id) = msg.reply_to {
                                if client_config
                                    .waiting_for_jobs
                                    .swap(None)
                                    .map_or(false, |x| id != *x)
                                {
                                    proceed = false;
                                }
                            };

                            if proceed {
                                for job in msg.jobs {
                                    accept_job(job, ws_send.clone(), client_config.clone()).await
                                }
                            }
                        }
                        ServerMsg::AbortJob(job) => {
                            let job_id = job.job_id;
                            let (inserted_send, inserted_recv) =
                                futures::channel::oneshot::channel();
                            let abort =
                                tokio::spawn(cancel_job(job, client_config.clone(), inserted_recv));
                            client_config
                                .cancelling_job_handles
                                .lock()
                                .await
                                .insert(job_id, abort);
                            let _ = inserted_send.send(());
                        }
                        ServerMsg::ServerHello => {
                            tracing::info!("Hi, server o/");
                        }
                    }
                }
            }
            Message::Ping(_) | Message::Pong(_) => (),
            _ => tracing::warn!("Unsupported message: {:?}", x),
        }
    }

    let _ = keepalive_handle.await;
    let _ = poll_jobs_handle.await;

    client_config.waiting_for_jobs.store(None);

    tracing::warn!("Disconnected!");
    ws_send
}
