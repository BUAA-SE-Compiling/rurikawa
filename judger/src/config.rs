use crate::tester::exec::ImageUsage;
use crate::tester::exec::{Capturable, Step, Test};
use futures::future::join_all;
use futures::stream::StreamExt;
use serde::{self, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeToml {
    pub id: String,
    pub jobs: HashMap<String, ImageUsage>,
}
