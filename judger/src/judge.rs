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
    pub time_limit: Option<usize>,
    pub mem_limit: Option<usize>,
    pub before_exec: Vec<Vec<String>>,
    pub exec: Vec<String>,
    pub expected_out: String,
    pub image_name: String,
}
