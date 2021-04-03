use crate::prelude::{CancellationToken, CancellationTokenHandle, FlowSnake};
use arc_swap::{ArcSwap, ArcSwapOption};
use bollard::Docker;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::AtomicBool,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::{sync::Mutex, task::JoinHandle};

use super::model::AbortJob;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub host: String,
    pub max_concurrent_tasks: usize,
    pub ssl: bool,
    pub access_token: Option<String>,
    pub register_token: Option<String>,
    pub alternate_name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub cache_folder: PathBuf,
    #[serde(default)]
    pub docker_config: Arc<DockerConfig>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            host: "".into(),
            max_concurrent_tasks: 1,
            ssl: false,
            access_token: None,
            register_token: None,
            alternate_name: None,
            tags: None,
            cache_folder: PathBuf::new(),
            docker_config: Arc::new(Default::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerConfig {
    /// The user every docker container should run in
    // docker_user_id: u16,

    /// CPU share available for image building use. This field will result
    /// in allowing the CPU to run `build_cpu_share * 100ms` in every 100ms
    /// CPU time.
    pub build_cpu_share: f64,

    /// CPU share available for running use. This field will be the upper limit
    /// of the load factor of all running task in the testing container.
    pub run_cpu_share: f64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        DockerConfig {
            build_cpu_share: 0.5,
            run_cpu_share: 0.3,
        }
    }
}

#[derive(Debug)]
pub struct SharedClientData {
    /// Configuration of this client
    pub cfg: ArcSwap<ClientConfig>,
    /// A unique id for all connection created by this client, similar to
    /// what `state` does in OAuth
    pub conn_id: u128,
    /// Number of running tests
    pub running_tests: AtomicUsize,
    /// The message id of the ongoing job request
    pub waiting_for_jobs: ArcSwapOption<FlowSnake>,
    /// Whether this client is aborting
    pub aborting: AtomicBool,
    /// HTTP client
    pub client: reqwest::Client,
    /// All test suites whose folder is being edited.
    pub locked_test_suite: dashmap::DashMap<FlowSnake, (u64, CancellationTokenHandle)>,
    /// Handle for all jobs currently running
    pub running_job_handles: Mutex<HashMap<FlowSnake, (JoinHandle<()>, CancellationTokenHandle)>>,
    /// Handle for all jobs currently cancelling
    pub cancelling_job_handles: Mutex<HashMap<FlowSnake, JoinHandle<()>>>,
    /// Information for currently-cancelling jobs.
    pub cancelling_job_info: dashmap::DashMap<FlowSnake, AbortJob>,
    /// Global cancellation token handle
    pub cancel_handle: CancellationTokenHandle,
    // /// The docker instance we're connecting
    // pub docker: Docker
}

impl SharedClientData {
    pub fn new(cfg: ClientConfig) -> SharedClientData {
        SharedClientData {
            cfg: ArcSwap::new(Arc::new(cfg)),
            conn_id: rand::random(),
            // WORKAROUND: Client hang issue in hyper crate.
            // see: https://github.com/hyperium/hyper/issues/2312
            client: reqwest::Client::builder()
                .pool_idle_timeout(std::time::Duration::from_secs(0))
                .pool_max_idle_per_host(0)
                .build()
                .unwrap(),
            aborting: AtomicBool::new(false),
            waiting_for_jobs: ArcSwapOption::new(None),
            running_tests: AtomicUsize::new(0),
            locked_test_suite: dashmap::DashMap::new(),
            running_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_info: DashMap::new(),
            cancel_handle: CancellationTokenHandle::new(),
        }
    }

    pub fn swap_cfg(&self, cfg: Arc<ClientConfig>) -> Arc<ClientConfig> {
        self.cfg.swap(cfg)
    }

    pub fn cfg(&self) -> arc_swap::Guard<Arc<ClientConfig>> {
        ArcSwap::load(&self.cfg)
    }

    pub fn cfg_ref(&self) -> Arc<ClientConfig> {
        ArcSwap::load_full(&self.cfg)
    }

    pub fn register_endpoint(&self) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/register", ssl, self.cfg().host)
    }

    pub fn verify_endpoint(&self) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/verify", ssl, self.cfg().host)
    }

    pub fn websocket_endpoint(&self) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("wss")
        } else {
            format_args!("ws")
        };

        if let Some(token) = &self.cfg().access_token {
            format!(
                "{}://{}/api/v1/judger/ws?token={}&conn={:x}",
                ssl,
                self.cfg().host,
                token,
                self.conn_id
            )
        } else {
            format!(
                "{}://{}/api/v1/judger/ws?conn={:x}",
                ssl,
                self.cfg().host,
                self.conn_id
            )
        }
    }

    pub fn test_suite_download_endpoint(&self, suite_id: FlowSnake) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!(
            "{}://{}/api/v1/judger/download-suite/{}",
            ssl,
            self.cfg().host,
            suite_id
        )
    }

    pub fn test_suite_info_endpoint(&self, suite_id: FlowSnake) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/tests/{}", ssl, self.cfg().host, suite_id)
    }

    pub fn result_upload_endpoint(&self) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/judger/upload", ssl, self.cfg().host)
    }

    pub fn result_send_endpoint(&self) -> String {
        let ssl = if self.cfg().ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/judger/result", ssl, self.cfg().host)
    }

    pub fn job_folder_root(&self) -> PathBuf {
        self.cfg().cache_folder.join("jobs")
    }

    pub fn test_suite_folder_root(&self) -> PathBuf {
        self.cfg().cache_folder.join("suites")
    }

    pub fn job_folder(&self, job_id: FlowSnake) -> PathBuf {
        self.job_folder_root().join(job_id.to_string())
    }

    pub fn test_suite_folder(&self, suite_id: FlowSnake) -> PathBuf {
        self.test_suite_folder_root().join(suite_id.to_string())
    }

    pub fn test_suite_folder_lockfile(&self, suite_id: FlowSnake) -> PathBuf {
        self.test_suite_folder_root()
            .join(format!("{}.lock", suite_id))
    }

    pub fn temp_file_folder_root(&self) -> PathBuf {
        self.cfg().cache_folder.join("files")
    }

    pub fn random_temp_file_path(&self) -> PathBuf {
        self.temp_file_folder_root()
            .join(FlowSnake::generate().to_string())
    }

    pub async fn obtain_suite_lock(&self, suite_id: FlowSnake) -> Option<CancellationTokenHandle> {
        let state = rand::random();
        let handle = CancellationTokenHandle::new();
        let entry = self
            .locked_test_suite
            .entry(suite_id)
            .or_insert_with(|| (state, handle.child_token()))
            .clone();
        tracing::debug!("Trying to obtain suite lock for {}", suite_id);
        if entry.0 == state {
            tracing::debug!("Lock obtained");
            Some(entry.1)
        } else {
            tracing::debug!("Already locked");
            (entry.1).cancelled().await;
            tracing::debug!("Lock cleared");
            None
        }
    }

    pub fn suite_unlock(&self, suite_id: FlowSnake) {
        self.locked_test_suite.remove(&suite_id);
        log::info!("Unlocked {}", suite_id);
    }

    pub fn new_job(&self) -> usize {
        self.running_tests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    pub fn finish_job(&self) -> usize {
        let res = self
            .running_tests
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        res - 1
    }
}
