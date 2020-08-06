pub use crate::tester::exec::{Image, ImageUsage, JudgeInfo};
use serde::{self, Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeToml {
    pub id: String,
    pub jobs: HashMap<String, ImageUsage>,
}
