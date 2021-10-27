use super::model::AbortJob;
use crate::prelude::{CancellationTokenHandle, FlowSnake};
use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::AtomicBool,
    sync::{atomic::AtomicUsize, Arc},
};
use tokio::{
    sync::{Mutex, OwnedMutexGuard, OwnedRwLockReadGuard, OwnedRwLockWriteGuard},
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
    /// The user every docker container should run in.
    pub docker_user: Option<String>,

    /// CPU share available for image building use. This field will result
    /// in allowing the CPU to run `build_cpu_share * 100ms` in every 100ms
    /// CPU time.
    pub build_cpu_share: Option<f64>,

    /// CPU share available for running use. This field will be the upper limit
    /// of the load factor of all running task in the testing container.
    pub run_cpu_share: Option<f64>,
}

impl Default for DockerConfig {
    fn default() -> Self {
        DockerConfig {
            docker_user: None,
            build_cpu_share: Some(0.5),
            run_cpu_share: Some(0.3),
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

    /// All test suites whose folder is being edited. The lock MUST be used internally.
    test_suite_modify: std::sync::Mutex<HashMap<FlowSnake, TestSuiteStatus>>,

    /// Handle for all jobs currently running
    pub running_job_handles: Mutex<HashMap<FlowSnake, (JoinHandle<()>, CancellationTokenHandle)>>,
    /// Handle for all jobs currently cancelling
    pub cancelling_job_handles: Mutex<HashMap<FlowSnake, JoinHandle<()>>>,
    /// Information for currently-cancelling jobs.
    pub cancelling_job_info: dashmap::DashMap<FlowSnake, AbortJob>,
    /// Global cancellation token handle
    pub abort_handle: CancellationTokenHandle,
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
            test_suite_modify: std::sync::Mutex::new(HashMap::new()),
            running_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_handles: Mutex::new(HashMap::new()),
            cancelling_job_info: DashMap::new(),
            abort_handle: CancellationTokenHandle::new(),
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

    pub async fn before_suite_might_modify(&self, id: FlowSnake) -> OwnedMutexGuard<()> {
        let arc = {
            let suites_map = self
                .test_suite_modify
                .lock()
                .expect("something panicked when locking this lock. Panic!");
            let suite = suites_map
                .get(&id)
                .expect("Test suite must be present before modifying");
            suite.modify.clone()
        };

        arc.lock_owned().await
    }

    pub async fn before_suite_modify(&self, id: FlowSnake) -> OwnedRwLockWriteGuard<()> {
        let arc = {
            let suites_map = self
                .test_suite_modify
                .lock()
                .expect("something panicked when locking this lock. Panic!");
            let suite = suites_map
                .get(&id)
                .expect("Test suite must be present before modifying");
            suite.update.clone()
        };

        arc.write_owned().await
    }

    pub async fn on_suite_run(&self, id: FlowSnake) -> OwnedRwLockReadGuard<()> {
        let arc = {
            let mut suites_map = self
                .test_suite_modify
                .lock()
                .expect("something panicked when locking this lock. Panic!");
            let suite = suites_map.entry(id).or_default();
            suite.update.clone()
        };

        arc.read_owned().await
    }

    /// Function to call before the job starts. Creates data for the corresponding test suites.
    #[must_use]
    pub fn before_job_start(self: Arc<Self>, id: FlowSnake) -> TestSuiteRunningGuard {
        let mut suites_map = self
            .test_suite_modify
            .lock()
            .expect("something panicked when locking this lock. Panic!");
        let suite = suites_map.entry(id).or_default();
        suite.rc.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

        drop(suites_map);
        TestSuiteRunningGuard {
            client_data: self,
            suite_id: id,
        }
    }

    fn suite_drop(&self, id: FlowSnake) {
        let mut suites_map = self
            .test_suite_modify
            .lock()
            .expect("something panicked when locking this lock. Panic!");
        let suite = match suites_map.get(&id) {
            Some(suite) => suite,
            None => {
                tracing::error!("Failed to access test suite {} while still holding a guard to it. Maybe a bug?",id);
                return;
            }
        };
        let remaining = suite.rc.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        if remaining == 0 {
            suites_map.remove(&id);
        }
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

/// Data structure to ensure that test suites are safe to modify.
///
/// The overall locking pattern looks like this:
///
/// ```plaintext
/// JobCreated
/// |                              rc += 1
/// CheckForTestSuiteUpdate -----> lock(modify) <- Other modifying tasks will
/// | |                                    |       wait here
/// | [UpdateSuite?]                       |
/// |   |                                  |
/// |   Yes              write_lock(update)| <- If any other task is running
/// |   | |                   |            |    they will wait here
/// |   | ...update...        v            |
/// |   |                unlock(update)    |
/// |   No                                 |
/// |                                      |
/// RunTestSuite ------> read_lock(update) |  <- Tasks should not wait here
/// |                          |           |
/// | <------------------------+---unlock(modify)
/// | |                        |
/// | ...run...                |
/// | ...run...                |
/// | |                        v
/// FinishRunning <----- unlock(update)
///                      rc -= 1
/// ```
#[derive(Debug, Default)]
pub struct TestSuiteStatus {
    /// Reference count of this test suite. If this reaches zero, the
    /// corresponding map entry should be deallocated.
    rc: AtomicUsize,
    /// Lock to obtain when trying to read or update test suite data.
    ///
    /// A read lock should be obtained for every job that has reached or passed
    /// `Compiling` phase and before `Finished`.
    ///
    /// A write lock should be obtained for every job that is trying to update
    /// the test suite data.
    ///
    /// This lock is wrapped inside an `Arc<T>` to allow owned access.
    update: Arc<tokio::sync::RwLock<()>>,
    /// Lock to obtain when trying to potentially modify test suite data.
    ///
    /// Every task should obtain a mutex before trying to check for test suite
    /// updates.
    ///
    /// This lock is wrapped inside an `Arc<T>` to allow owned access.
    modify: Arc<tokio::sync::Mutex<()>>,
}

pub struct TestSuiteRunningGuard {
    client_data: Arc<SharedClientData>,
    suite_id: FlowSnake,
}

impl Drop for TestSuiteRunningGuard {
    fn drop(&mut self) {
        self.client_data.suite_drop(self.suite_id);
    }
}
