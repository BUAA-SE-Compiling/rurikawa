use async_trait::async_trait;
use derive_builder::Builder;
use rquickjs::IntoJsByRef;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

/// The result returned by running a subprocess.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, IntoJsByRef)]
pub struct ProcessOutput {
    pub ret_code: i32,
    pub command: String,
    pub stdout: String,
    pub stderr: String,

    pub runned_inside: String,
}

#[derive(Debug, Clone)]
pub enum OutputComparisonSource {
    File(PathBuf),
    InMemory(String),
}

/// One step in testing
#[derive(Debug, Clone)]
pub struct ExecStep {
    /// Environment variables to set
    pub env: Arc<HashMap<String, String>>,
    /// The command to run
    pub run: String,
    /// The target to compare output with
    pub compare_output_with: Option<OutputComparisonSource>,
}

/// A group of exec that are done in the same container
#[derive(Clone)]
pub struct ExecGroup {
    /// Container to run in
    pub run_in: Arc<dyn CommandRunner>,
    /// Run steps
    pub steps: Vec<ExecStep>,
}

impl std::fmt::Debug for ExecGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecGroup")
            .field("run_in", &self.run_in.name())
            .field("steps", &self.steps)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub commands: Vec<ExecGroup>,
}

#[async_trait]
pub trait CommandRunner {
    fn name(&self) -> std::borrow::Cow<'static, str>;
    async fn run(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
        opt: &CommandRunOptions,
    ) -> anyhow::Result<ProcessOutput>;
}

#[derive(Debug, Default, Builder)]
#[builder(setter(into, strip_option))]
pub struct CommandRunOptions {
    #[builder(default = "100*1024")]
    pub stdout_size_limit: usize,

    #[builder(default = "100*1024")]
    pub stderr_size_limit: usize,

    /// Timeout in milliseconds
    #[builder(default)]
    pub timeout: Option<u64>,
}
