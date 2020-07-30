use crate::tester::exec::{Capturable, Step, Test};
use crate::tester::{runner::DockerCommandRunner, JobFailure};
use futures::stream::StreamExt;
use names::{Generator, Name};
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
    /// Build (or pull) a image with the specified config, return the image name.
    pub async fn build_image(&self, instance: bollard::Docker) -> String {
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
                tag.to_owned()
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
                tag.to_owned()
            }
        }
    }
}

/// Info on the building process and the usage of a "ready-to-use" image,
/// which contains the compiler to be examined.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageUsage {
    pub image: Image,
    pub build: Vec<Vec<String>>,
    pub run: Vec<Vec<String>>,
}

/// Extra info on how to turn `ImageUsage` into `docker` usage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestInfo {
    pub name: String,
    pub recursive: bool,
    pub test_cases: PathBuf,
    /// The command needed to run the VM, so as to finally perform an I/O check
    pub run_vm: Vec<String>,
    pub docker_config: TestDockerConfig,
    /// A Map between file placeholders and file paths.
    pub env_map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestDockerConfig {
    pub volume: HashMap<String, String>,
}

/// A collection of all the `TestJob`s.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestSuite {
    pub jobs: Vec<TestJob>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestJob {
    /// Time limit of a step, in seconds.
    pub time_limit: Option<usize>,
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    /// List of commands to be executed.
    pub exec: Vec<Vec<String>>,
    /// Expected `stdout` of the last command.
    pub expected_out: String,
    pub image_name: String,
}

impl TestJob {
    pub async fn run(&self, instance: bollard::Docker) -> Result<(), JobFailure> {
        // TODO: Use the mem_limit field
        let mut names = Generator::with_naming(Name::Numbered);
        let mut runner = DockerCommandRunner::new(
            instance,
            &names.next().unwrap(),
            &self.image_name,
            self.mem_limit,
        )
        .await;
        let mut t = Test::new();

        self.exec.iter().for_each(|step| {
            t.add_step(Step::new_with_timeout(
                Capturable::new(step.to_vec()),
                self.time_limit
                    .map(|n| std::time::Duration::from_secs(n as u64)),
            ));
        });

        t.expected(&self.expected_out);
        t.run(&mut runner).await?;
        Ok(())
    }
}
