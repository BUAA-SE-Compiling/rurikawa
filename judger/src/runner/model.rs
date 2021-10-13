use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::tester::ProcessInfo;
use async_trait::async_trait;

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
    pub commands: Vec<ExecStep>,
}

#[async_trait]
pub trait CommandRunner {
    fn name(&self) -> std::borrow::Cow<'static, str>;
    async fn run(
        &self,
        command: &str,
        env: &mut (dyn Iterator<Item = (&str, &str)> + Send),
    ) -> anyhow::Result<ProcessInfo>;
}
