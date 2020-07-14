use serde::{Deserialize, Serialize};
use serde_derive::*;

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
