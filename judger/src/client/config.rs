use crate::prelude::{CancellationTokenHandle, FlowSnake};
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
    pub cfg: ClientConfig,
    pub running_tests: AtomicUsize,
    pub aborting: AtomicBool,
    pub client: reqwest::Client,
    /// All test suites whose folder is being edited.
    pub locked_test_suite: RwLock<HashMap<FlowSnake, Arc<Mutex<()>>>>,
    pub running_job_handles: Mutex<HashMap<FlowSnake, (JoinHandle<()>, CancellationTokenHandle)>>,
    pub cancelling_job_handles: Mutex<HashMap<FlowSnake, JoinHandle<()>>>,
    pub cancel_handle: CancellationTokenHandle,
}

impl SharedClientData {
    pub fn new(cfg: ClientConfig) -> SharedClientData {
        SharedClientData {
            cfg,
            client: reqwest::Client::new(),
            aborting: AtomicBool::new(false),
            running_tests: AtomicUsize::new(0),
            locked_test_suite: RwLock::new(HashMap::new()),
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
                "{}://{}/api/v1/judger/ws?token={}",
                ssl, self.cfg.host, token
            )
        } else {
            format!("{}://{}/api/v1/judger/ws", ssl, self.cfg.host)
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

    pub async fn obtain_suite_lock(&self, suite_id: FlowSnake) -> Arc<Mutex<()>> {
        let cur = self.locked_test_suite.read().await.get(&suite_id).cloned();
        if let Some(cur) = cur {
            cur
        } else {
            let arc = Arc::new(Mutex::new(()));
            self.locked_test_suite
                .write()
                .await
                .insert(suite_id, arc.clone());
            arc
        }
    }

    pub async fn suite_unlock(&self, suite_id: FlowSnake) {
        self.locked_test_suite.write().await.remove(&suite_id);
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
