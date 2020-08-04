use crate::tester::exec::{Capturable, Step, Test};
use crate::tester::{
    runner::{DockerCommandRunner, DockerCommandRunnerOptions},
    JobFailure,
};
use futures::future::join_all;
use futures::stream::StreamExt;
use serde::{self, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeToml {
    pub id: String,
    pub jobs: HashMap<String, ImageUsage>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "source")]
pub enum Image {
    /// An existing image.
    Image { tag: String },
    /// An image to be built with a Dockerfile.
    Dockerfile { tag: String, path: String },
}

impl Image {
    pub fn tag(&self) -> String {
        match &self {
            Image::Image { tag, .. } => tag.to_owned(),
            Image::Dockerfile { tag, .. } => tag.to_owned(),
        }
    }

    /// Build (or pull) a image with the specified config.
    pub async fn build(&self, instance: bollard::Docker) {
        match &self {
            Image::Image { tag } => {
                instance
                    .create_image(
                        Some(bollard::image::CreateImageOptions {
                            from_image: tag.to_owned(),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .map(|mr| {
                        mr.unwrap_or_else(|e| {
                            panic!("Failed to pull Docker image `{}`: {}", tag, e)
                        })
                    })
                    .collect::<Vec<_>>()
                    .await;
            }
            Image::Dockerfile { tag, path } => {
                instance
                    .build_image(
                        bollard::image::BuildImageOptions {
                            dockerfile: path.to_owned(),
                            t: tag.to_owned(),
                            rm: true,
                            ..Default::default()
                        },
                        None,
                        None,
                    )
                    .map(|mr| {
                        mr.unwrap_or_else(|e| {
                            panic!("Failed to build Docker image `{}`: {}", tag, e)
                        })
                    })
                    .collect::<Vec<_>>()
                    .await;
            }
        }
    }
}

/// Info on the building process and the usage of a "ready-to-use" image,
/// which contains the compiler to be examined.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageUsage {
    pub image: Image,
    /// The sequence of commands to build
    pub build: Vec<Vec<String>>,
    pub run: Vec<Vec<String>>,
}

/// Extra info on how to turn `ImageUsage` into `docker` usage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestInfo {
    pub name: String,
    pub test_cases: PathBuf,
    /// The command needed to run the VM, so as to finally perform an I/O check
    pub run_vm: Vec<String>,
    // TODO: Use this field.
    pub docker_config: TestDockerConfig,
    /// A Map between file placeholders and file paths.
    pub env_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestDockerConfig {
    pub volume: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestCase {
    pub uid: u32,
    /// List of commands to be executed.
    pub exec: Vec<Vec<String>>,
    /// Expected `stdout` of the last command.
    pub expected_out: String,
}

/// A suite of `Test`s to be run.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestSuite {
    /// Time limit of a step, in seconds.
    pub time_limit: Option<usize>,
    // TODO: Use this field.
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    /// The test contents.
    pub test_cases: Vec<TestCase>,
    /// The image which contains the compiler to be tested.
    pub image_name: String,
}

impl TestSuite {
    pub fn add_case(&mut self, case: TestCase) {
        self.test_cases.push(case)
    }

    pub async fn run(&self, instance: bollard::Docker) -> Vec<Result<(), JobFailure>> {
        let runner = DockerCommandRunner::new(
            instance,
            Image::Image {
                tag: self.image_name.clone(),
            },
            DockerCommandRunnerOptions {
                container_name: self.image_name.clone(),
                mem_limit: self.mem_limit,
                ..Default::default()
            },
        )
        .await;

        let res: Vec<_> = self
            .test_cases
            .iter()
            .map(|case| {
                let mut t = Test::new();
                case.exec.iter().for_each(|step| {
                    t.add_step(Step::new_with_timeout(
                        Capturable::new(step.to_vec()),
                        self.time_limit
                            .map(|n| std::time::Duration::from_secs(n as u64)),
                    ));
                });
                t.expected(&case.expected_out);
                t.run(&runner)
            })
            .collect();

        join_all(res).await
    }
}
