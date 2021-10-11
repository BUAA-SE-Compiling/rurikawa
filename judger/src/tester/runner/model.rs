use std::{collections::HashMap, sync::Arc};

use crate::tester::ProcessInfo;
use async_trait::async_trait;

/// One step in testing
pub struct ExecStep {
    /// Container to run in
    pub run_in: Arc<dyn CommandRunner>,
    /// Environment variables to set
    pub env: Arc<HashMap<String, String>>,
    /// The command to run
    pub run: String,
}

pub struct TestCase {
    pub commands: Vec<ExecStep>,
}

#[async_trait]
pub trait CommandRunner {
    async fn run(&self, command: String) -> ProcessInfo;
}
