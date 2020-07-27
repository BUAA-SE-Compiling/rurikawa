use serde::{self, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeToml {
    pub id: String,
    pub job: HashMap<String, JudgeJobConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeJobConfig {
    pub image: ImageConfig,
    pub build: Vec<Vec<String>>,
    pub run: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "source")]
pub enum ImageConfig {
    Image {
        tag: String,
    },
    Dockerfile {
        folder: PathBuf,
        dockerfile: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestConfig {
    pub name: String,
    pub recursive: bool,
    pub test_cases: PathBuf,
    pub run: Vec<Vec<String>>,
    pub docker_config: TestDockerConfig,
    pub file_env_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestDockerConfig {
    pub volume: HashMap<String, String>,
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
