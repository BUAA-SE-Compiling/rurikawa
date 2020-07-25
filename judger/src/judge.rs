use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeToml {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeConfig {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ImageConfig {
    Remote(String),
    Dockerfile(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobConfig {
    /// Time limit of EACH step, in seconds.
    pub time_limit: Option<usize>,
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    pub before_exec: Vec<Vec<String>>,
    pub exec: Vec<String>,
    pub expected_out: String,
    pub image_name: String,
}
