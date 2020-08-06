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
            let mut cmd = vec![$prog.to_string()];
            $(
                cmd.push($arg.to_string());
            )*
            cmd
        }
    };
}

#[macro_export]
macro_rules! bash {
    ( $script:expr ) => {{
        let mut cmd = vec!["bash".to_owned(), "-c".to_owned()];
        cmd.push($script.to_string());
        cmd
    }};
}

#[macro_export]
macro_rules! sh {
    ( $script:expr ) => {{
        let mut cmd = vec!["sh".to_owned(), "-c".to_owned()];
        cmd.push($script.to_string());
        cmd
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

    //? Should `runner` be mutable?
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
    // /// The sequence of commands to build
    // pub build: Vec<Vec<String>>,
    pub run: Vec<Vec<String>>,
}

/// Extra info on how to turn `ImageUsage` into `docker` usage.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgeInfo {
    /// Directory of tests.
    pub dir: PathBuf,
    /// File names of tests.
    pub tests: Vec<String>,
    /// Variables and extensions of test files
    /// (`$src`, `$bin`, `$stdin`, `$stdout`, etc...).
    /// For example: `"$src" => ".go"`.
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
    /// List of commands to be executed.
    pub exec: Vec<Vec<String>>,
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
    pub image_name: String,
}

impl TestSuite {
    pub fn try_new(info: JudgeInfo, usage: ImageUsage, opt: TestSuiteOptions) -> Result<Self> {
        let TestSuiteOptions {
            time_limit,
            mem_limit,
            ..
        } = opt;
        let test_cases = info
            .tests
            .iter()
            .map(|test| -> Result<TestCase> {
                let mut test_path = info.dir.clone();
                test_path.push(test);
                let replacer: HashMap<String, _> = info
                    .vars
                    .iter()
                    .map(|(var, ext)| {
                        (var.to_owned(), {
                            let mut p = test_path.clone();
                            p.set_extension(ext);
                            p
                        })
                    })
                    .collect();
                let exec: Vec<Vec<String>> = usage
                    .run
                    .iter()
                    .map(|cmd| {
                        cmd.iter()
                            .map(|word| {
                                if let Some(replacement) = replacer.get(word) {
                                    replacement.to_str().unwrap().to_owned()
                                } else {
                                    word.to_owned()
                                }
                            })
                            .collect()
                    })
                    .collect();
                let mut expected_out = "".to_owned();
                let mut file = fs::File::open(replacer.get("$stdout").ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        "Output verification failed, no `$stdout` in dictionary",
                    )
                })?)?;
                file.read_to_string(&mut expected_out)?;
                Ok(TestCase { exec, expected_out })
            })
            .collect::<Result<Vec<TestCase>>>()?;
        Ok(TestSuite {
            time_limit,
            mem_limit,
            image_name: usage.image.tag(),
            test_cases,
        })
    }

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
                let runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    Image::Image {
                        tag: "alpine:latest".to_owned(),
                    },
                    DockerCommandRunnerOptions {
                        build_image: true,
                        ..Default::default()
                    },
                )
                .await;
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

    #[test]
    fn ok() {
        todo!()
    }
}
