use super::runner::CommandRunner;
use super::{utils::diff, BuildError};
use crate::{
    client::model::ResultUploadConfig,
    client::model::{upload_test_result, TestResult, TestResultKind},
    prelude::*,
};
use anyhow::Result;
use async_compat::CompatExt;
use bollard::models::{BuildInfo, Mount};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use path_absolutize::Absolutize;
use path_slash::PathBufExt;
use serde::{self, Deserialize, Serialize};
use std::path::Path;
use std::time;
use std::{collections::HashMap, io, path::PathBuf, string::String, sync::Arc};
use tokio::io::{AsyncReadExt, BufWriter};
use tokio::sync::mpsc::UnboundedSender;

/// A Host-to-container volume binding for the container.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bind {
    /// Absolute/Relative `from` path (in the host machine).
    pub from: PathBuf,
    /// Absolute `to` path (in the container).
    pub to: PathBuf,
    /// Extra options for this bind. Leave a new `String` for empty.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub readonly: bool,
}

impl Bind {
    pub fn canonical_from(&mut self, base_dir: &Path) {
        let mut from_base = base_dir.to_owned();
        from_base.push(&self.from);
        self.from = from_base.absolutize().unwrap().into_owned();
    }

    pub fn to_mount(&self) -> Mount {
        Mount {
            target: Some(self.to.display().to_string()),
            source: Some(self.from.display().to_string()),
            typ: Some(bollard::models::MountTypeEnum::BIND),
            read_only: self.readonly.into(),
            ..Default::default()
        }
    }
}

pub fn path_canonical_from(path: &Path, base_dir: &Path) -> PathBuf {
    let mut from_base = base_dir.to_owned();
    from_base.push(path);
    from_base.absolutize().unwrap().into_owned()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "source")]
#[serde(rename_all = "camelCase")]
pub enum Image {
    /// An existing image.
    Image { tag: String },
    /// An image to be built with a Dockerfile.
    Dockerfile {
        /// Name to be assigned to the image.
        tag: String,
        /// Path of the context directory, relative to the context directory.
        path: PathBuf,
        /// Path of the dockerfile itself, relative to the context directory.
        /// Leaving this value to None means using the default dockerfile: `path/Dockerfile`.
        file: Option<PathBuf>,
    },
}

/// The definition of a test case
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseDefinition {
    pub name: String,
    pub should_fail: bool,
    pub has_out: bool,
}

/// Judger's public config, specific to a paticular repository,
/// Maintained by the owner of the project to be tested.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JudgerPublicConfig {
    pub time_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub name: String,
    pub test_groups: HashMap<String, Vec<TestCaseDefinition>>,
    /// Variables and extensions of test files
    /// (`$src`, `$bin`, `$stdin`, `$stdout`, etc...).
    /// For example: `"$src" => "go"`.
    pub vars: HashMap<String, String>,
    /// Sequence of commands necessary to perform an IO check.
    pub run: Vec<String>,
    /// The path of test root directory to be mapped inside test container
    pub mapped_dir: Bind,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<Bind>>,
}

/// Judger's private config, specific to a host machine.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgerPrivateConfig {
    /// Directory of test sources files (including `stdin` and `stdout` files)
    /// outside the container.
    pub test_root_dir: PathBuf,
    /// Directory of test sources files inside the container.
    pub mapped_test_root_dir: PathBuf,
}

/// The public representation of a test.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestCase {
    /// File name of the test case.
    pub name: String,
    /// List of commands to be executed.
    pub exec: Vec<String>,
    /// Expected `stdout` of the last command.
    pub expected_out: Option<String>,
    /// Should this test case fail
    pub should_fail: bool,
}

/// Initialization options for `Testsuite`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestSuiteOptions {
    /// File names of tests.
    pub tests: Vec<String>,
    /// Time limit of a step, in seconds.
    pub time_limit: Option<usize>,
    // TODO: Use this field.
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    /// If the image needs to be built before run.
    pub build_image: bool,
    /// If the image needs to be removed after run.
    pub remove_image: bool,
}

impl Default for TestSuiteOptions {
    fn default() -> Self {
        TestSuiteOptions {
            tests: vec![],
            time_limit: None,
            mem_limit: None,
            build_image: false,
            remove_image: false,
        }
    }
}
