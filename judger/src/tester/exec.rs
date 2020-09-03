use super::utils::diff;
use super::{
    runner::{CommandRunner, DockerCommandRunner, DockerCommandRunnerOptions},
    ExecError, ExecErrorKind, JobFailure, OutputMismatch, ProcessInfo,
};
use crate::{
    client::model::{upload_test_result, TestResult, TestResultKind},
    prelude::*,
};
use anyhow::Result;
use futures::stream::StreamExt;
use serde::{self, Deserialize, Serialize};
use std::fs;
use std::io::{self, prelude::*};
use std::time;
use std::{collections::HashMap, path::PathBuf, string::String};

#[cfg(unix)]
use super::utils::strsignal;

#[cfg(not(unix))]
fn strsignal(i: i32) -> String {
    "".into()
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
        /// Path of the dockerfile itself, relative to the context directory.
        /// Leaving this value to None means using the default dockerfile: `Dockerfile`.
        file: Option<PathBuf>,
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
            Image::Dockerfile { tag, path, file } => {
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
                            dockerfile: file
                                .as_ref()
                                .map(|x| x.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Dockerfile".into()),
                            t: tag.into(),
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

    /// Remove the Image when finished.
    /// Attention: this action must be done AFTER removing related containers.
    pub async fn remove(self, instance: bollard::Docker) -> Result<()> {
        let tag = self.tag();
        instance
            .remove_image(
                &tag,
                Some(bollard::image::RemoveImageOptions {
                    ..Default::default()
                }),
                None,
            )
            .await?;
        Ok(())
    }
}

/// A Host-to-container volume binding for the container.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Bind {
    /// Absolute/Relative `from` path (in the host machine).
    from: PathBuf,
    /// Absolute `to` path (in the container).
    to: PathBuf,
    /// Extra options for this bind. Leave a new `String` for empty.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    options: String,
}

impl Bind {
    /// Generate a `host-src:container-dest[:options]` string for the binding.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub fn stringify(&self) -> String {
        fn strip_quote(s: &str) -> Option<&str> {
            s.strip_prefix("\"")?.strip_suffix("\"")
        }
        let Bind { from, to, options } = self;
        let from = format!("{:?}", std::fs::canonicalize(from).unwrap());
        let from = strip_quote(&from).unwrap();
        let to = format!("{:?}", to);
        let to = strip_quote(&to).unwrap();
        let mut res = format!("{}:{}", from, to);
        dbg!(&res);
        if !options.is_empty() {
            res.push_str(&format!(":{}", options));
        }
        res
    }
}

/// Judger's public config, specific to a paticular repository,
/// Maintained by the owner of the project to be tested.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgerPublicConfig {
    pub name: String,
    /// Variables and extensions of test files
    /// (`$src`, `$bin`, `$stdin`, `$stdout`, etc...).
    /// For example: `"$src" => "go"`.
    pub vars: HashMap<String, String>,
    /// Sequence of commands necessary to perform an IO check.
    pub run: Vec<String>,
    /// The path of test root directory to be mapped inside test container
    pub mapped_dir: PathBuf,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<Bind>>,
}

/// Judger's private config, specific to a host machine.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgerPrivateConfig {
    /// Directory of test sources files (including `stdin` and `stdout` files)
    /// in the container.
    pub test_root_dir: PathBuf,
}

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
#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// A suite of `TestCase`s to be run.
///
/// Attention: a `TestSuite` instance should NOT be constructed manually.
/// Please use `TestSuite::from_config`, for example.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestSuite {
    /// The test contents.
    pub test_cases: Vec<TestCase>,
    /// The image which contains the compiler to be tested.
    image: Option<Image>,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<String>>,
    /// Initialization options for `Testsuite`.
    pub options: TestSuiteOptions,
}

impl TestSuite {
    pub fn add_case(&mut self, case: TestCase) {
        self.test_cases.push(case)
    }

    pub fn from_config(
        image: Image,
        private_cfg: JudgerPrivateConfig,
        public_cfg: JudgerPublicConfig,
        options: TestSuiteOptions,
    ) -> Result<Self> {
        let test_cases = options
            .tests
            .iter()
            .map(|name| -> Result<TestCase> {
                let mut mapped_dir = public_cfg.mapped_dir.clone();
                let mut test_root = private_cfg.test_root_dir.clone();
                mapped_dir.push(name);
                test_root.push(name);
                let replacer: HashMap<String, _> = public_cfg
                    .vars
                    .iter()
                    .map(|(var, ext)| {
                        (var.to_owned(), {
                            // Special case for `$stdout`:
                            // These variables will point to files under `io_dir`,
                            // while others to `src_dir`.
                            let mut p = match var.as_ref() {
                                "$stdout" => test_root.clone(),
                                _ => mapped_dir.clone(),
                            };
                            p.set_extension(ext);
                            p
                        })
                    })
                    .collect();
                let exec: Vec<String> = public_cfg
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
            image: Some(image),
            test_cases,
            options,
            binds: public_cfg
                .binds
                .map(|bs| bs.iter().map(|b| b.stringify()).collect()),
        })
    }

    pub async fn run(
        &mut self,
        instance: bollard::Docker,
        result_channel: Option<tokio::sync::mpsc::UnboundedSender<(String, TestResult)>>,
        upload_info: Option<(&str, reqwest::Client)>,
    ) -> anyhow::Result<HashMap<String, TestResult>> {
        let TestSuiteOptions {
            time_limit,
            mem_limit,
            build_image,
            remove_image,
            ..
        } = self.options;

        // Take ownership of the `Image` instance stored in `Self`
        let image = std::mem::replace(&mut self.image, None)
            .expect("TestSuite instance not fully constructed");
        let image_tag = image.tag();
        let runner = DockerCommandRunner::try_new(instance, image, {
            DockerCommandRunnerOptions {
                mem_limit,
                build_image,
                remove_image,
                binds: self.binds.clone(),
                ..Default::default()
            }
        })
        .await
        .unwrap_or_else(|e| panic!("Failed to create command runner `{}`: {}", &image_tag, e));

        // TODO: Remove drain when this compiler issue gets repaired:
        // https://github.com/rust-lang/rust/issues/64552
        let res = futures::stream::iter(self.test_cases.drain(..))
            .map(|case| {
                let upload_info = upload_info.clone();
                let result_channel = result_channel.clone();
                let runner = &runner;
                async move {
                    result_channel.as_ref().map(|ch| {
                        ch.send((
                            case.name.clone(),
                            TestResult {
                                kind: TestResultKind::Running,
                                result_file_id: None,
                            },
                        ))
                    });
                    let mut t = Test::new();
                    case.exec.iter().for_each(|step| {
                        t.add_step(Step::new_with_timeout(
                            Capturable::new(sh![step]),
                            time_limit.map(|n| std::time::Duration::from_secs(n as u64)),
                        ));
                    });
                    t.expected(&case.expected_out);
                    let res = t.run(runner).await;
                    let res = upload_test_result(res, upload_info).await;
                    result_channel
                        .as_ref()
                        .map(|ch| ch.send((case.name.clone(), res.clone())));
                    (case.name.clone(), res)
                }
            })
            .buffer_unordered(16)
            .collect::<HashMap<_, _>>()
            .await;

        runner.kill().await;

        Ok(res)
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
                    bollard::Docker::connect_with_local_defaults().unwrap(),
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
    fn golem_no_volume() -> Result<()> {
        block_on(async {
            let image_name = "golem_no_volume";
            // Repo directory in the host FS.
            let host_repo_dir = PathBuf::from(r"../golem");

            let mut ts = TestSuite::from_config(
                Image::Dockerfile {
                    tag: image_name.to_owned(),
                    path: host_repo_dir,
                    file: None,
                },
                JudgerPrivateConfig {
                    test_root_dir: PathBuf::from(r"../golem/src"),
                },
                JudgerPublicConfig {
                    name: "golem_no_volume".into(),
                    mapped_dir: PathBuf::from(r"golem/src"),
                    binds: None,
                    run: [
                        "cd golem",
                        "python ./golemc.py $src -o $bin",
                        "cat $stdin | python ./golem.py $bin",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
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
                TestSuiteOptions {
                    tests: ["succ"].iter().map(|s| s.to_string()).collect(),
                    time_limit: None,
                    mem_limit: None,
                    build_image: true,
                    remove_image: true,
                },
            )?;

            let instance = bollard::Docker::connect_with_local_defaults().unwrap();
            ts.run(instance, None, None).await;
            Ok(())
        })
    }

    #[test]
    fn golem_with_volume() -> Result<()> {
        block_on(async {
            let image_name = "golem";
            // Repo directory in the host FS.
            let host_repo_dir = PathBuf::from(r"../golem");

            let mut ts = TestSuite::from_config(
                Image::Dockerfile {
                    tag: image_name.to_owned(),
                    path: host_repo_dir, // public: c# gives repo remote, rust clone and unzip
                    file: None,
                },
                JudgerPrivateConfig {
                    test_root_dir: PathBuf::from(r"../golem/src"), // private
                },
                JudgerPublicConfig {
                    name: "golem".into(),
                    binds: Some(vec![Bind {
                        from: PathBuf::from(r"../golem/src"), // private
                        to: PathBuf::from(r"/src"),           // private
                        options: "ro".to_owned(),
                    }]),
                    mapped_dir: PathBuf::from(r"/src"), // private
                    run: [
                        "cd golem",
                        "python ./golemc.py $src -o $bin",
                        "cat $stdin | python ./golem.py $bin",
                    ] // public
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                    vars: [
                        ("$src", "py"),
                        ("$bin", "pyc"),
                        ("$stdin", "in"),
                        ("$stdout", "out"),
                    ] // public
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                },
                TestSuiteOptions {
                    tests: ["succ"].iter().map(|s| s.to_string()).collect(), // private
                    time_limit: None,                                        // private
                    mem_limit: None,                                         // private
                    build_image: true,                                       // private
                    remove_image: true,                                      // private
                },
            )?;

            let instance = bollard::Docker::connect_with_local_defaults().unwrap();
            ts.run(instance, None, None).await;
            Ok(())
        })
    }
}
