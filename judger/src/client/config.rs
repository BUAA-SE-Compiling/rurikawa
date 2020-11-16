use crate::prelude::{CancellationToken, CancellationTokenHandle, FlowSnake};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, path::PathBuf, sync::atomic::AtomicBool, sync::atomic::AtomicUsize,
    sync::Arc,
};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
};

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
        }
    }
}

#[derive(Debug)]
pub struct SharedClientData {
    /// Configuration of this client
    pub cfg: ClientConfig,
    /// A unique id for all connection created by this client, similar to
    /// what `state` does in OAuth
    pub conn_id: u128,
    /// Number of running tests
    pub running_tests: AtomicUsize,
    /// Whether this client is aborting
    pub aborting: AtomicBool,
    /// HTTP client
    pub client: reqwest::Client,
    /// All test suites whose folder is being edited.
    pub locked_test_suite: dashmap::DashMap<FlowSnake, CancellationToken>,
    /// Handle for all jobs currently running
    pub running_job_handles: Mutex<HashMap<FlowSnake, (JoinHandle<()>, CancellationTokenHandle)>>,
    /// Handle for all jobs currently cancelling
    pub cancelling_job_handles: Mutex<HashMap<FlowSnake, JoinHandle<()>>>,
    /// Global cancellation token handle
    pub cancel_handle: CancellationTokenHandle,
}

impl SharedClientData {
    pub fn new(cfg: ClientConfig) -> SharedClientData {
        SharedClientData {
            cfg,
            conn_id: rand::random(),
            // WORKAROUND: Client hang issue in hyper crate.
            // see: https://github.com/hyperium/hyper/issues/2312
            client: reqwest::Client::builder()
                .pool_idle_timeout(std::time::Duration::from_secs(0))
                .pool_max_idle_per_host(0)
                .build()
                .unwrap(),
            aborting: AtomicBool::new(false),
            running_tests: AtomicUsize::new(0),
            locked_test_suite: dashmap::DashMap::new(),
            running_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_handles: Mutex::new(HashMap::new()),
            cancel_handle: CancellationTokenHandle::new(),
        }
    }

    pub fn register_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/register", ssl, self.cfg.host)
    }

    pub fn verify_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };

        format!("{}://{}/api/v1/judger/verify", ssl, self.cfg.host)
    }

    pub fn websocket_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("wss")
        } else {
            format_args!("ws")
        };

        if let Some(token) = &self.cfg.access_token {
            format!(
                "{}://{}/api/v1/judger/ws?token={}&conn={:x}",
                ssl, self.cfg.host, token, self.conn_id
            )
        } else {
            format!(
                "{}://{}/api/v1/judger/ws?conn={:x}",
                ssl, self.cfg.host, self.conn_id
            )
        }
    }

    pub fn test_suite_download_endpoint(&self, suite_id: FlowSnake) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!(
            "{}://{}/api/v1/judger/download-suite/{}",
            ssl, self.cfg.host, suite_id
        )
    }

    pub fn test_suite_info_endpoint(&self, suite_id: FlowSnake) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/tests/{}", ssl, self.cfg.host, suite_id)
    }

    pub fn result_upload_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/judger/upload", ssl, self.cfg.host)
    }

    pub fn result_send_endpoint(&self) -> String {
        let ssl = if self.cfg.ssl {
            format_args!("https")
        } else {
            format_args!("http")
        };
        format!("{}://{}/api/v1/judger/result", ssl, self.cfg.host)
    }

    pub fn job_folder_root(&self) -> PathBuf {
        let mut job_temp_folder = self.cfg.cache_folder.clone();
        job_temp_folder.push("jobs");
        job_temp_folder
    }

    pub fn test_suite_folder_root(&self) -> PathBuf {
        let mut test_suite_temp_folder = self.cfg.cache_folder.clone();
        test_suite_temp_folder.push("suites");
        test_suite_temp_folder
    }

    pub fn job_folder(&self, job_id: FlowSnake) -> PathBuf {
        let mut job_temp_folder = self.job_folder_root();
        job_temp_folder.push(job_id.to_string());
        job_temp_folder
    }

    pub fn test_suite_folder(&self, suite_id: FlowSnake) -> PathBuf {
        let mut test_suite_temp_folder = self.test_suite_folder_root();
        test_suite_temp_folder.push(suite_id.to_string());
        test_suite_temp_folder
    }

    pub fn test_suite_folder_lockfile(&self, suite_id: FlowSnake) -> PathBuf {
        let mut test_suite_temp_folder = self.test_suite_folder_root();
        test_suite_temp_folder.push(format!("{}.lock", suite_id));
        test_suite_temp_folder
    }

    pub fn temp_file_folder_root(&self) -> PathBuf {
        let mut test_suite_temp_folder = self.cfg.cache_folder.clone();
        test_suite_temp_folder.push("files");
        test_suite_temp_folder
    }

    pub fn random_temp_file_path(&self) -> PathBuf {
        let mut root = self.temp_file_folder_root();
        let random_filename = FlowSnake::generate().to_string();
        root.push(random_filename);
        root
    }

    pub async fn obtain_suite_lock(&self, suite_id: FlowSnake) -> Option<CancellationTokenHandle> {
        let handle = CancellationTokenHandle::new();
        let entry = self
            .locked_test_suite
            .entry(suite_id)
            .or_insert_with(|| handle.get_token())
            .clone();
        if entry.is_token_of(&handle) {
            Some(handle)
        } else {
            entry.await;
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
