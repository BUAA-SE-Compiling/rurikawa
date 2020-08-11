use super::utils::diff;
#[cfg(unix)]
use super::utils::strsignal;
use super::{
    runner::{CommandRunner, DockerCommandRunner, DockerCommandRunnerOptions},
    ExecError, ExecErrorKind, JobFailure, OutputMismatch, ProcessInfo,
};
use crate::prelude::*;
use anyhow::Result;
use futures::future::join_all;
use futures::stream::StreamExt;
use serde::{self, Deserialize, Serialize};
use std::fs;
use std::io::{self, prelude::*};
use std::time;
use std::{collections::HashMap, path::PathBuf};

#[cfg(not(unix))]
fn strsignal(i: i32) -> String {
    return "".into();
}

#[macro_export]
macro_rules! command {
    ( $prog:expr, $( $arg:expr ),* ) => {
        {
            vec![
                $prog.to_string(),
                $($arg.to_string(),)*
            ]
        }
    };
}

#[macro_export]
macro_rules! bash {
    ( $script:expr ) => {{
        vec!["bash".to_string(), "-c".to_string(), $script.to_string()]
    }};
}

#[macro_export]
macro_rules! sh {
    ( $script:expr ) => {{
        vec!["sh".to_string(), "-c".to_string(), $script.to_string()]
    }};
}

pub struct Capturable(Vec<String>);

impl Capturable {
    pub fn new(cmd: Vec<String>) -> Self {
        Capturable(cmd)
    }

    async fn capture<R: CommandRunner + Send>(self, runner: &R) -> PopenResult<ProcessInfo> {
        let Self(cmd) = self;
        runner.run(&cmd).await
    }
}

/// One step in a `Test`.
pub struct Step {
    /// The command to be executed.
    pub cmd: Capturable,
    /// The timeout of the command.
    pub timeout: Option<time::Duration>,
}

impl Step {
    /// Make a new `Step` with no timeout.
    pub fn new(cmd: Capturable) -> Self {
        Step { cmd, timeout: None }
    }

    /// Set `timeout` for a `Step`.
    pub fn timeout(mut self, timeout: time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Make a new `Step` with a `timeout`.
    pub fn new_with_timeout(cmd: Capturable, timeout: Option<time::Duration>) -> Self {
        Step { cmd, timeout }
    }

    /// Run the `Step` and collect its info, considering the `timeout`.
    pub async fn capture<R>(self, runner: &R) -> PopenResult<ProcessInfo>
    where
        R: CommandRunner + Send,
    {
        if let Some(timeout) = self.timeout {
            let mres = tokio::time::timeout(timeout, self.cmd.capture(runner)).await;
            if let Ok(res) = mres {
                res
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Popen capture timed out",
                ))
            }
        } else {
            self.cmd.capture(runner).await
        }
    }
}

/// A particular multi-`Step` test.
/// An I/O match test against `expected` is performed at the last `Step`
#[derive(Default)]
pub struct Test {
    steps: Vec<Step>,
    /// The expected `stdout` content.
    expected: Option<String>,
}

impl Test {
    pub fn new() -> Self {
        Test {
            steps: vec![],
            expected: None,
        }
    }

    pub fn add_step(&mut self, step: Step) -> &mut Self {
        self.steps.push(step);
        self
    }

    pub fn set_steps(&mut self, steps: Vec<Step>) -> &mut Self {
        self.steps = steps;
        self
    }

    pub fn expected(&mut self, expected: &str) -> &mut Self {
        self.expected = Some(expected.to_owned());
        self
    }

    // ? Should `runner` be mutable?
    pub async fn run<R>(self, runner: &R) -> Result<(), JobFailure>
    where
        R: CommandRunner + Send,
    {
        let expected = self.expected.expect("Run Failed: Expected String not set");
        let mut output: Vec<ProcessInfo> = vec![];
        let steps_len = self.steps.len();
        for (i, step) in self.steps.into_iter().enumerate() {
            let info = match step.capture(runner).await {
                Ok(res) => res,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::TimedOut,
                        output,
                    }))
                }
                Err(e) => panic!("Run Failed: Cannot launch subprocess, {}", e),
            };

            output.push(info.clone());
            let code = info.ret_code;
            let is_unix = cfg!(unix);
            match () {
                _ if code > 0 || (code < 0 && !is_unix) => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::ReturnCodeCheckFailed,
                        output,
                    }));
                }
                _ if code < 0 && is_unix => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::RuntimeError(format!(
                            "Runtime Error: {}",
                            strsignal(-code)
                        )),
                        output,
                    }));
                }
                _ => (),
            }

            // Special case for last step
            if i == steps_len - 1 {
                let got = info.stdout;
                if got != expected {
                    return Err(JobFailure::OutputMismatch(OutputMismatch {
                        diff: diff(&got, &expected),
                        output,
                    }));
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "source")]
pub enum Image {
    /// An existing image.
    Image { tag: String },
    /// An image to be built with a Dockerfile.
    Dockerfile {
        /// Name to be assigned to the image.
        tag: String,
        /// Path of the context directory.
        path: PathBuf,
    },
}

impl Image {
    pub fn tag(&self) -> String {
        match &self {
            Image::Image { tag, .. } => tag.to_owned(),
            Image::Dockerfile { tag, .. } => tag.to_owned(),
        }
    }

    /// Build (or pull) a image with the specified config.
    pub async fn build(&self, instance: bollard::Docker) -> Result<()> {
        match &self {
            Image::Image { tag } => {
                let ms = instance
                    .create_image(
                        Some(bollard::image::CreateImageOptions {
                            from_image: tag.to_owned(),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .collect::<Vec<_>>()
                    .await;
                // ! FIXME: This is not efficient (for not being lazy),
                // ! but it seems that directly collecting to Result is not possible.
                ms.into_iter().collect::<Result<Vec<_>, _>>().map_err(|e| {
                    JobFailure::internal_err_from(format!("Failed to pull image `{}`: {}", tag, e))
                })?;
                Ok(())
            }
            Image::Dockerfile { tag, path } => {
                let tar = {
                    let buffer: Vec<u8> = vec![];
                    let mut builder = tar::Builder::new(buffer);
                    builder.append_dir_all(".", path)?;
                    let bytes = builder.into_inner();
                    hyper::Body::wrap_stream(futures::stream::iter(vec![bytes]))
                };
                let ms = instance
                    .build_image(
                        bollard::image::BuildImageOptions {
                            dockerfile: "Dockerfile",
                            t: tag,
                            rm: true,
                            ..Default::default()
                        },
                        None,
                        // Freeze `path` as a tar archive.
                        Some(tar),
                    )
                    .collect::<Vec<_>>()
                    .await;
                ms.into_iter().collect::<Result<Vec<_>, _>>().map_err(|e| {
                    JobFailure::internal_err_from(format!("Failed to build image `{}`: {}", tag, e))
                })?;
                Ok(())
            }
        }
    }
}

/// Info on the building process and the usage of a "ready-to-use" image,
/// which contains the compiler to be examined.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageUsage {
    /// The image to be used.
    pub image: Image,
    /// Sequence of commands necessary to perform an IO check.
    pub run: Vec<String>,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<String>>,
}

/// Extra info on how to turn `ImageUsage` into `docker` usage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeInfo {
    /// Directory of test sources in the container.
    pub src_dir: PathBuf,
    /// Directory of test IO files on the host machine.
    pub io_dir: PathBuf,
    /// File names of tests.
    pub tests: Vec<String>,
    /// Variables and extensions of test files
    /// (`$src`, `$bin`, `$stdin`, `$stdout`, etc...).
    /// For example: `"$src" => "go"`.
    pub vars: HashMap<String, String>,
}

/*
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestDockerConfig {
    pub volumes: HashMap<String, String>,
}
*/

/// The public representation of a test.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestCase {
    /// File name of the test case.
    pub name: String,
    /// List of commands to be executed.
    pub exec: Vec<String>,
    /// Expected `stdout` of the last command.
    pub expected_out: String,
}

/// Initialization options for `Testsuite`.
#[derive(Debug, Clone, Default)]
pub struct TestSuiteOptions {
    /// Time limit of a step, in seconds.
    pub time_limit: Option<usize>,
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    /// If the image needs to be built before run.
    pub build_image: bool,
}

/// A suite of `Testcase`s to be run.
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
    pub image: Image,
    /// If the image needs to be pulled/built before run.
    pub build_image: bool,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<String>>,
}

impl TestSuite {
    pub fn new(image: Image, build_image: bool) -> Self {
        TestSuite {
            time_limit: None,
            mem_limit: None,
            test_cases: vec![],
            image,
            build_image,
            binds: None,
        }
    }

    pub fn add_case(&mut self, case: TestCase) {
        self.test_cases.push(case)
    }

    pub fn from_config(info: JudgeInfo, usage: ImageUsage, opt: TestSuiteOptions) -> Result<Self> {
        let TestSuiteOptions {
            time_limit,
            mem_limit,
            build_image,
            ..
        } = opt;
        let test_cases = info
            .tests
            .iter()
            .map(|name| -> Result<TestCase> {
                let mut src_dir = info.src_dir.clone();
                src_dir.push(name);
                let mut io_dir = info.io_dir.clone();
                io_dir.push(name);
                let replacer: HashMap<String, _> = info
                    .vars
                    .iter()
                    .map(|(var, ext)| {
                        (var.to_owned(), {
                            // Special case for `$stdout`:
                            // These variables will point to files under `io_dir`,
                            // while others to `src_dir`.
                            let mut p = match var.as_ref() {
                                "$stdout" => io_dir.clone(),
                                _ => src_dir.clone(),
                            };
                            p.set_extension(ext);
                            p
                        })
                    })
                    .collect();
                let exec: Vec<String> = usage
                    .run
                    .iter()
                    .map(|line| {
                        replacer.iter().fold(line.to_owned(), |seed, (pat, rep)| {
                            seed.replace(pat, &format!(r#""{}""#, rep.to_str().unwrap()))
                        })
                    })
                    .collect();
                let mut expected_out = "".to_owned();
                let stdout_path = replacer.get("$stdout").ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "Output verification failed, no `$stdout` in dictionary",
                    )
                })?;
                // ? QUESTION: Now I'm reading `$stdout` in host, but the source file, etc. are handled in containers.
                // ? Is this desirable?
                let mut file = fs::File::open(stdout_path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!(
                            "Output verification failed, failed to open `{:?}`: {}",
                            stdout_path, e,
                        ),
                    )
                })?;
                file.read_to_string(&mut expected_out)?;
                Ok(TestCase {
                    name: name.to_owned(),
                    exec,
                    expected_out,
                })
            })
            .collect::<Result<Vec<TestCase>>>()?;
        Ok(TestSuite {
            time_limit,
            mem_limit,
            image: usage.image,
            test_cases,
            build_image,
            binds: usage.binds,
        })
    }

    pub async fn run(&self, instance: bollard::Docker) -> Vec<Result<(), JobFailure>> {
        let image_tag = self.image.tag();
        let runner = DockerCommandRunner::try_new(
            instance,
            self.image.clone(),
            DockerCommandRunnerOptions {
                mem_limit: self.mem_limit,
                build_image: self.build_image,
                binds: self.binds.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap_or_else(|e| panic!("Failed to create command runner `{}`: {}", &image_tag, e));

        let res: Vec<_> = self
            .test_cases
            .iter()
            .map(|case| {
                let mut t = Test::new();
                case.exec.iter().for_each(|step| {
                    t.add_step(Step::new_with_timeout(
                        Capturable::new(sh![step]),
                        self.time_limit
                            .map(|n| std::time::Duration::from_secs(n as u64)),
                    ));
                });
                t.expected(&case.expected_out);
                t.run(&runner)
            })
            .collect();

        let res = join_all(res).await;
        runner.kill().await;
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq as pretty_eq;
    use tokio_test::block_on;

    #[cfg(test)]
    #[cfg(unix)]
    mod tokio_runner {
        use super::*;
        use crate::tester::runner::TokioCommandRunner;

        #[test]
        fn ok() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $1}'"
                ))));
                t.expected("Hello,\n");
                let res = t.run(&TokioCommandRunner {}).await;
                assert!(matches!(dbg!(res), Ok(())));
            })
        }

        #[test]
        fn error_code() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' && false"
                ))));
                t.expected("Goodbye, world!");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::ReturnCodeCheckFailed,
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 1,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' && false\"]"
                                .into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn signal() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    // "ping www.bing.com & sleep 0.5; kill $!",
                    r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::RuntimeError(
                        format!(
                            "Runtime Error: {}",
                            strsignal(15)
                        )
                    ),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: -15,
                            command: "[\"sh\", \"-c\", \"{ sleep 0.1; kill $$; } & i=0; while [ \\\"$i\\\" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done\"]".into(),
                            stdout: "0\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_mismatch() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $2}'"
                ))));
                t.expected("Hello,\nworld!");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n- ".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\"]".into(),
                            stdout: "world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_timed_out() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(
                    Step::new(Capturable::new(sh!("echo 0; sleep 3; echo 1")))
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::TimedOut,
                    output: vec![ProcessInfo {
                        ret_code: 0,
                        command: "[\"echo\", \"This does nothing.\"]".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    }],
                }));
                pretty_eq!(got, expected);
            })
        }
    }

    mod docker_runner {
        use super::*;
        use crate::tester::runner::{DockerCommandRunner, DockerCommandRunnerOptions};

        fn docker_run<F, O>(f: F)
        where
            F: FnOnce(DockerCommandRunner, Test) -> O,
            O: futures::Future<Output = DockerCommandRunner>,
        {
            block_on(async {
                let runner = DockerCommandRunner::try_new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    Image::Image {
                        tag: "alpine:latest".to_owned(),
                    },
                    DockerCommandRunnerOptions {
                        build_image: true,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
                let t = Test::new();
                f(runner, t).await.kill().await;
            });
        }

        #[test]
        fn ok() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $1}'"
                ))));
                t.expected("Hello,\n");
                let res = t.run(&runner).await;
                assert!(matches!(dbg!(res), Ok(())));
                runner
            });
        }

        #[test]
        fn error_code() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' && false"
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::ReturnCodeCheckFailed,
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 1,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' && false\"]"
                                .into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
                runner
            })
        }

        #[test]
        fn signal() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    // "ping www.bing.com & sleep 0.5; kill $!",
                    r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::RuntimeError(
                        format!(
                            "Runtime Error: {}",
                            strsignal(15)
                        )
                    ),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: -15,
                            command: "[\"sh\", \"-c\", \"{ sleep 0.1; kill $$; } & i=0; while [ \\\"$i\\\" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done\"]".into(),
                            stdout: "0\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
                runner
            })
        }

        #[test]
        fn output_mismatch() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $2}'"
                ))));
                t.expected("Hello,\nworld!");
                let got = t.run(&runner).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n- ".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\"]".into(),
                            stdout: "world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
                runner
            })
        }

        #[test]
        fn output_timed_out() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(
                    Step::new(Capturable::new(sh!("echo 0; sleep 3; echo 1")))
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::TimedOut,
                    output: vec![ProcessInfo {
                        ret_code: 0,
                        command: "[\"echo\", \"This does nothing.\"]".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    }],
                }));
                pretty_eq!(got, expected);
                runner
            })
        }
    }
}

#[cfg(test)]
mod test_suite {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn golem() -> Result<()> {
        block_on(async {
            let image_name = "golem";
            // Repo directory in the host FS.
            let host_repo_dir = PathBuf::from(r"../golem");
            // Directories in the container FS.
            let repo_dir = PathBuf::from(r"golem");
            let mut tests_dir = repo_dir.clone();
            tests_dir.push("tests");

            let ts = TestSuite::from_config(
                JudgeInfo {
                    src_dir: PathBuf::from(r"golem/tests"),
                    io_dir: PathBuf::from(r"../golem/tests"),
                    tests: ["succ"].iter().map(|s| s.to_string()).collect(),
                    vars: [
                        ("$src", "py"),
                        ("$bin", "pyc"),
                        ("$stdin", "in"),
                        ("$stdout", "out"),
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                },
                ImageUsage {
                    image: Image::Dockerfile {
                        tag: image_name.to_owned(),
                        path: host_repo_dir,
                    },
                    run: [
                        "cd golem",
                        "python ./golemc.py $src -o $bin",
                        "cat $stdin | python ./golem.py $bin",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                    binds: None,
                },
                TestSuiteOptions {
                    time_limit: None,
                    mem_limit: None,
                    build_image: true,
                },
            )?;

            let instance = bollard::Docker::connect_with_unix_defaults().unwrap();
            ts.run(instance).await;
            Ok(())
        })
    }
}
