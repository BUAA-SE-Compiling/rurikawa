use super::{model::*, spj};
use super::{
    runner::{CommandRunner, DockerCommandRunner, DockerCommandRunnerOptions},
    ExecError, ExecErrorKind, JobFailure, OutputMismatch, ProcessInfo, ShouldFailFailure,
};
use super::{utils::diff, BuildError};
use crate::{
    client::model::ResultUploadConfig,
    client::model::{upload_test_result, TestResult, TestResultKind},
    config::JudgeTomlTestConfig,
    prelude::*,
};
use anyhow::Result;
use async_compat::CompatExt;
use bollard::models::{BuildInfo, Mount};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use path_slash::PathBufExt;
use regex::{Captures, Regex};
use rquickjs::{FromJs, IntoJsByRef};
use serde::de::value;
use std::path::Path;
use std::time;
use std::{collections::HashMap, io, path::PathBuf, string::String, sync::Arc};
use tokio::io::{AsyncReadExt, BufWriter};
use tokio::sync::mpsc::UnboundedSender;

#[cfg(unix)]
use super::utils::strsignal;

#[cfg(not(unix))]
fn strsignal(_i: i32) -> String {
    "".into()
}

#[macro_export]
macro_rules! command {
    ( $prog:expr, $( $arg:expr ),* ) => {
        {
            &[
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

pub struct Capturable(String);

impl Capturable {
    pub fn new(cmd: String) -> Self {
        Capturable(cmd)
    }

    /// Run the command represented by `self` with the given `runner` and `variables`
    async fn capture<R: CommandRunner + Send>(
        self,
        runner: &R,
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo> {
        runner.run(&self.0, variables).await
    }
}

/// One step in a `Test`.
pub struct Step {
    /// The command to be executed.
    pub cmd: Capturable,
    /// The command is created by the user, not the admin.
    pub is_user_command: bool,
    /// The timeout of the command.
    pub timeout: Option<time::Duration>,
}

impl Step {
    /// Make a new `Step` with no timeout.
    pub fn new(cmd: Capturable, is_user_command: bool) -> Self {
        Step {
            cmd,
            is_user_command,
            timeout: None,
        }
    }

    /// Set `timeout` for a `Step`.
    pub fn timeout(mut self, timeout: time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Make a new `Step` with a `timeout`.
    pub fn new_with_timeout(
        cmd: Capturable,
        timeout: Option<time::Duration>,
        is_user_command: bool,
    ) -> Self {
        Step {
            cmd,
            is_user_command,
            timeout,
        }
    }

    /// Run the `Step` and collect its info, considering the `timeout`.
    pub async fn capture<R>(
        self,
        runner: &R,
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo>
    where
        R: CommandRunner + Send,
    {
        let is_user_command = self.is_user_command;
        if let Some(timeout) = self.timeout {
            let mres = tokio::time::timeout(timeout, self.cmd.capture(runner, variables)).await;
            if let Ok(res) = mres {
                res.map(|mut i| {
                    i.is_user_command = is_user_command;
                    i
                })
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Popen capture timed out",
                ))
            }
        } else {
            self.cmd.capture(runner, variables).await.map(|mut i| {
                i.is_user_command = is_user_command;
                i
            })
        }
    }
}

static EOF_PATTERN: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\r?\n").unwrap());

/// A particular multi-`Step` test.
/// An I/O match test against `expected` is performed at the last `Step`
#[derive(Default)]
pub struct Test {
    steps: Vec<Step>,
    /// The expected `stdout` content.
    expected: Option<String>,
    /// Should this test fail?
    should_fail: bool,
}

impl Test {
    pub fn new() -> Self {
        Test {
            steps: vec![],
            expected: None,
            should_fail: false,
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
    pub async fn run<R>(
        self,
        runner: &R,
        variables: &HashMap<String, String>,
    ) -> Result<(), JobFailure>
    where
        R: CommandRunner + Send,
    {
        let mut output: Vec<ProcessInfo> = vec![];
        let steps_len = self.steps.len();
        let mut test_failed = false;
        for (i, step) in self.steps.into_iter().enumerate() {
            let info = match step.capture(runner, variables).await {
                Ok(res) => res,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::TimedOut,
                        output,
                    }))
                }
                Err(e) => return Err(JobFailure::InternalError(e.to_string())),
            };

            output.push(info.clone());
            let code = info.ret_code;
            let is_unix = cfg!(unix);
            match () {
                _ if (code > 0 || (code != 0 && !is_unix)) => {
                    if self.should_fail {
                        // bail out of test, but it's totally fine
                        test_failed = true;
                        break;
                    } else {
                        return Err(JobFailure::ExecError(ExecError {
                            stage: i,
                            kind: ExecErrorKind::ReturnCodeCheckFailed,
                            output,
                        }));
                    }
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
                if let Some(expected) = self.expected.as_ref() {
                    // * Actually there is a test that should not have passed,
                    // * because the `.out` file is missing a `\n`.
                    // * We trim the result here anyway...
                    let got = EOF_PATTERN.replace_all(info.stdout.trim(), "\n");
                    let expected = EOF_PATTERN.replace_all(expected.trim(), "\n");
                    let (different, diff_str) = diff(&got, &expected);
                    if different {
                        return Err(JobFailure::OutputMismatch(OutputMismatch {
                            diff: diff_str,
                            output,
                        }));
                    }
                }
            }
        }

        // Tests that _should_ fail but did not should return error here
        if self.should_fail && !test_failed {
            return Err(JobFailure::ShouldFail(ShouldFailFailure { output }));
        }

        Ok(())
    }
}

pub type BuildResultChannel = UnboundedSender<BuildInfo>;

impl Image {
    pub fn set_dockerfile_tag(&mut self, new_tag: String) {
        match self {
            Image::Image { .. } => {}
            Image::Dockerfile { tag, .. } => *tag = new_tag,
        }
    }

    pub fn tag(&self) -> String {
        match &self {
            Image::Image { tag, .. } => tag.to_owned(),
            Image::Dockerfile { tag, .. } => tag.to_owned(),
        }
    }

    pub fn replace_with_absolute_dir(&mut self, base_dir: PathBuf) {
        match self {
            Image::Image { .. } => {}
            Image::Dockerfile { path, file, .. } => {
                // if let Some(file) = file {
                //     let mut file_base = base_dir.clone();
                //     file_base.push(&file);
                //     *file = file_base;
                // }
                let mut path_base = base_dir;
                path_base.push(&path);
                *path = path_base;
            }
        }
    }

    /// Build (or pull) a image with the specified config.
    pub async fn build(
        &self,
        instance: bollard::Docker,
        partial_result_channel: Option<BuildResultChannel>,
        cancel: CancellationToken,
    ) -> Result<(), BuildError> {
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
                    .map(|x| match x {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    })
                    .collect::<Vec<_>>()
                    .with_cancel(cancel)
                    .await
                    .ok_or(BuildError::Cancelled)?;
                // ! FIXME: This is not efficient (for not being lazy),
                // ! but it seems that directly collecting to Result is not possible.
                ms.into_iter().collect::<Result<Vec<_>, _>>().map_err(|e| {
                    BuildError::ImagePullFailure(format!("Failed to pull image `{}`: {}", tag, e))
                })?;
                Ok(())
            }
            Image::Dockerfile { tag, path, file } => {
                let from_path = path.clone();
                let (pipe_recv, pipe_send) = async_pipe::pipe();
                let read_codec = tokio_util::codec::BytesCodec::new();
                let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);
                let task = async move {
                    let mut tar = async_tar::Builder::new(BufWriter::new(pipe_recv).compat());
                    match tar.append_dir_all(".", from_path).await {
                        Ok(_) => tar.finish().await,
                        e @ Err(_) => e,
                    }
                };

                enum BuildResult {
                    Success,
                    Error(String, Option<bollard::models::ErrorDetail>),
                }

                let task = tokio::spawn(task);
                let result = instance
                    .build_image(
                        bollard::image::BuildImageOptions {
                            dockerfile: file
                                .as_ref()
                                .map(|x| x.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Dockerfile".into()),
                            t: tag.into(),
                            rm: true,
                            forcerm: true,
                            ..Default::default()
                        },
                        None,
                        // Freeze `path` as a tar archive.
                        Some(hyper::Body::wrap_stream(frame)),
                    )
                    .map(|x| {
                        // TODO: wait for PR#107 to merge in bollard
                        match x {
                            Ok(info) => {
                                if let Some(e) = info.error {
                                    return Ok(BuildResult::Error(e, info.error_detail));
                                }
                                if let Some(ch) = partial_result_channel.as_ref() {
                                    let _ = ch.send(info);
                                }
                                Ok(BuildResult::Success)
                            }
                            Err(e) => Err(e),
                        }
                    })
                    .fold(Ok(BuildResult::Success), |last, x| async {
                        match (last, x) {
                            (Ok(last), Ok(BuildResult::Success)) => Ok(last),
                            (Ok(_), Ok(e @ BuildResult::Error(..))) => Ok(e),
                            (Ok(_), Err(e)) => Err(e),
                            (e @ Err(_), _) => e,
                        }
                    })
                    .with_cancel(cancel.clone())
                    .await
                    .ok_or(BuildError::Cancelled)?
                    .map_err(|e| BuildError::Internal(e.to_string()))?;

                if let BuildResult::Error(err, detail) = result {
                    return Err(BuildError::BuildError { error: err, detail });
                }

                task.await
                    .map_err(|e| BuildError::Internal(e.to_string()))?
                    .map_err(|e| BuildError::FileTransferError(e.to_string()))?;

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

// pub type JudgerPublicConfig = crate::client::model::TestSuite;

/// A suite of `TestCase`s to be run.
///
/// Attention: a `TestSuite` instance should NOT be constructed manually.
/// Please use `TestSuite::from_config`, for example.
pub struct TestSuite {
    /// The test contents.
    pub test_cases: Vec<TestCase>,
    /// The image which contains the compiler to be tested.
    image: Option<Image>,
    /// `host-src:container-dest` volume bindings for the container.
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    pub binds: Option<Vec<Mount>>,
    ///`(source, dest)` pairs of data to be copied into the container.
    pub copies: Option<Vec<(String, String)>>,
    /// Initialization options for `Testsuite`.
    pub options: TestSuiteOptions,
    /// Command to execute within each test case. Commands should be regular shell commands
    /// inside a unix shell.
    pub exec: Vec<RawStep>,
    /// Variables to be expanded at testing.
    ///
    /// Variables in this field are in the form of `{"$var": "dest"}`, which for example then
    /// expands `$var` inside test case `123` to `123.dest`.
    pub vars: HashMap<String, String>,

    /// Root folder inside **this** machine
    pub test_root: PathBuf,
    /// Root folder inside **container**
    pub container_test_root: PathBuf,

    /// Special Judger environment
    spj_env: Option<spj::SpjEnvironment>,
}

impl TestSuite {
    pub fn add_case(&mut self, case: TestCase) {
        self.test_cases.push(case)
    }

    /// Build the test suite from given configs.
    pub async fn from_config(
        image: Image,
        base_dir: &Path,
        private_cfg: JudgerPrivateConfig,
        public_cfg: JudgerPublicConfig,
        job_cfg: &JudgeTomlTestConfig,
        options: TestSuiteOptions,
    ) -> Result<Self> {
        let container_test_root = private_cfg.mapped_test_root_dir.clone();
        let test_root = private_cfg.test_root_dir.clone();

        let index = construct_case_index(&public_cfg);
        let raw_steps = job_cfg
            .run
            .iter()
            .map(|s| RawStep {
                command: s.to_owned(),
                is_user_command: true,
            })
            .chain(public_cfg.run.iter().map(|s| RawStep {
                command: s.to_owned(),
                is_user_command: false,
            }))
            .collect();

        // create test cases
        // TODO: Remove drain when this compiler issue gets repaired:
        // https://github.com/rust-lang/rust/issues/64552
        let mut test_cases = futures::stream::iter(options.tests.clone().drain(..))
            .map(|name| {
                let public_cfg = &public_cfg;
                let test_root = &test_root;
                let container_test_root = &container_test_root;
                let case = index.get(&name).unwrap();
                create_test_case(public_cfg, test_root, container_test_root, case, name)
            })
            .buffer_unordered(16)
            .collect::<Vec<Result<TestCase>>>()
            .await;
        let test_cases = test_cases.drain(..).collect::<Result<Vec<_>>>()?;

        let spj = if let Some(script) = public_cfg.special_judge_script {
            let script_path = test_root.join(script);
            Some(spj::make_spj(&script_path).await?)
        } else {
            None
        };

        Ok(TestSuite {
            image: Some(image),
            test_cases,
            options,
            exec: raw_steps,
            vars: public_cfg.vars,
            binds: public_cfg.binds.map(|bs| {
                bs.iter()
                    .map(|b| {
                        let mut b = b.clone();
                        b.canonical_from(base_dir);
                        b.to_mount()
                    })
                    .collect()
            }),
            copies: Some(vec![(
                path_canonical_from(&public_cfg.mapped_dir.from, base_dir).to_slash_lossy(),
                public_cfg.mapped_dir.to.to_slash_lossy(),
            )]),
            spj_env: spj,
            test_root,
            container_test_root,
        })
    }

    pub async fn run(
        &mut self,
        instance: bollard::Docker,
        base_dir: PathBuf,
        build_result_channel: Option<BuildResultChannel>,
        result_channel: Option<tokio::sync::mpsc::UnboundedSender<(String, TestResult)>>,
        upload_info: Option<Arc<ResultUploadConfig>>,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<HashMap<String, TestResult>> {
        let rnd_id = rand::random::<u32>();
        let TestSuiteOptions {
            time_limit,
            mem_limit,
            build_image,
            remove_image,
            ..
        } = self.options;

        log::trace!("{:08x}: started", rnd_id);

        // Take ownership of the `Image` instance stored in `Self`
        let mut image = self
            .image
            .take()
            .expect("TestSuite instance not fully constructed");
        image.replace_with_absolute_dir(base_dir);
        image.set_dockerfile_tag(format!("{}_{:08x}", image.tag(), rnd_id));
        let runner = DockerCommandRunner::try_new(
            instance,
            image,
            {
                DockerCommandRunnerOptions {
                    mem_limit,
                    build_image,
                    remove_image,
                    binds: self.binds.clone(),
                    copies: self.copies.clone(),
                    cancellation_token: cancellation_token.clone(),
                    ..Default::default()
                }
            },
            build_result_channel,
        )
        .await?;

        log::trace!("{:08x}: runner created", rnd_id);

        let mut result = HashMap::new();

        for case in &self.test_cases {
            log::info!(
                "{:08x}: started test: {}, timeout {:?}",
                rnd_id,
                case.name,
                time_limit
            );

            result_channel.as_ref().map(|ch| {
                ch.send((
                    case.name.clone(),
                    TestResult {
                        kind: TestResultKind::Running,
                        score: None,
                        result_file_id: None,
                    },
                ))
            });
            let mut t = Test::new();
            t.should_fail = case.should_fail;
            self.exec.iter().for_each(|step| {
                t.add_step(Step::new_with_timeout(
                    Capturable::new(step.command.clone()),
                    time_limit.map(|n| std::time::Duration::from_secs(n as u64)),
                    step.is_user_command,
                ));
            });
            if let Some(out) = case.expected_out.as_deref() {
                t.expected(out);
            }

            let replacer: HashMap<String, _> = self
                .vars
                .iter()
                .map(|(var, ext)| {
                    (var.to_owned(), {
                        // Special case for `$stdout`:
                        // These variables will point to files under `io_dir`,
                        // while others to `src_dir`.
                        let mut p = match var.as_ref() {
                            "$stdout" => self.test_root.clone(),
                            _ => self.container_test_root.clone(),
                        };
                        p.push(format!("{}.{}", &case.name, ext));
                        p.to_slash_lossy()
                    })
                })
                .collect();

            log::trace!("{:08x}: created test: {}", rnd_id, case.name);

            let res = t
                .run(&runner, &replacer)
                .with_cancel(cancellation_token.clone())
                .await
                .ok_or(JobFailure::Cancelled)
                .and_then(|x| x);

            log::trace!("{:08x}: runned: {}", rnd_id, case.name);

            let (mut res, cache) = TestResult::from_failure(res);
            if let Some(cfg) = &upload_info {
                if let Some(cache) = cache {
                    let file = upload_test_result(cache, cfg.clone(), &case.name).await;
                    res.result_file_id = file;
                }
            }

            log::trace!("{:08x}: uploaded result: {}", rnd_id, case.name);

            result_channel
                .as_ref()
                .map(|ch| ch.send((case.name.clone(), res.clone())));

            result.insert(case.name.clone(), res);
        }

        runner.kill().await;

        log::trace!("{:08x}: finished", rnd_id);

        Ok(result)
    }
}

async fn create_test_case(
    public_cfg: &JudgerPublicConfig,
    test_root: &PathBuf,
    container_test_root: &PathBuf,
    case: &TestCaseDefinition,
    name: String,
) -> Result<TestCase> {
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
                    _ => container_test_root.clone(),
                };
                p.push(format!("{}.{}", name, ext));
                p.to_slash_lossy()
            })
        })
        .collect();

    // ? QUESTION: Now I'm reading `$stdout` in host, but the source file, etc. are handled in containers.
    // ? Is this desirable?

    let expected_out = if case.has_out && !case.should_fail {
        let stdout_path = replacer.get("$stdout").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Output verification failed, no `$stdout` in dictionary",
            )
        })?;

        let mut expected_out = Vec::new();
        let mut file = tokio::fs::File::open(stdout_path).await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Output verification failed, failed to open `{:?}`: {}",
                    stdout_path, e,
                ),
            )
        })?;
        file.read_to_end(&mut expected_out).await?;
        Some(String::from_utf8_lossy(&expected_out).into_owned())
    } else {
        None
    };

    Result::Ok(TestCase {
        name: name.to_owned(),
        expected_out,
        should_fail: case.should_fail,
    })
}

fn construct_case_index(pub_cfg: &JudgerPublicConfig) -> HashMap<String, &TestCaseDefinition> {
    let mut idx = HashMap::new();

    for group in pub_cfg.test_groups.values() {
        for test_case in group {
            idx.insert(test_case.name.clone(), test_case);
        }
    }

    idx
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' | awk '{print $1}'".into()),
                    true,
                ));
                t.expected("Hello,\n");
                let res = t.run(&TokioCommandRunner {}).await;
                assert!(matches!(dbg!(res), Ok(())));
            })
        }

        #[test]
        fn error_code() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' && false".into()),
                    true,
                ));
                t.expected("Goodbye, world!");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::ReturnCodeCheckFailed,
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                            is_user_command: true,
                        },
                        ProcessInfo {
                            ret_code: 1,
                            command: "echo 'Hello, world!' && false".into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                            is_user_command: true,
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
                            is_user_command:true,
                            command: r"echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: -15,
                            is_user_command:true,
                            command:r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#.into(),
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' | awk '{print $2}'".into()),
                    true,
                ));
                t.expected("Hello,\nworld!");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            is_user_command: true,
                            command: r"echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            is_user_command: true,
                            command: "echo 'Hello, world!' | awk '{print $2}'".into(),
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(
                    Step::new(Capturable::new("echo 0; sleep 3; echo 1".into()), true)
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::TimedOut,
                    output: vec![ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: r"echo 'This does nothing.'".into(),
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
                    Option::<BuildResultChannel>::None,
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' | awk '{print $1}'".into()),
                    true,
                ));
                t.expected("Hello,\n");
                let res = t.run(&runner, &HashMap::new()).await;
                assert!(matches!(dbg!(res), Ok(())));
                runner
            });
        }

        #[test]
        fn error_code() {
            docker_run(|runner, mut t| async {
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' && false".into()),
                    true,
                ));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner, &HashMap::new()).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::ReturnCodeCheckFailed,
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                            is_user_command: true,
                        },
                        ProcessInfo {
                            ret_code: 1,
                            command: "echo 'Hello, world!' && false".into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                            is_user_command: true,
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(Capturable::new(
                    // Kill a running task
                    r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#.into()
                ),true));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner, &HashMap::new()).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: if cfg!(unix){ ExecErrorKind::RuntimeError(
                        format!(
                            "Runtime Error: {}",
                            strsignal(15)
                        )
                    )}else{
                        ExecErrorKind::ReturnCodeCheckFailed
                    },
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            is_user_command:true,
                            command: r"echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: -15,
                            is_user_command:true,
                            command:r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#.into(),
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(Step::new(
                    Capturable::new("echo 'Hello, world!' | awk '{print $2}'".into()),
                    true,
                ));
                t.expected("Hello,\nworld!");
                let got = t.run(&runner, &HashMap::new()).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            is_user_command: true,
                            command: r"echo 'This does nothing.'".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            is_user_command: true,
                            command: "echo 'Hello, world!' | awk '{print $2}'".into(),
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
                t.add_step(Step::new(
                    Capturable::new(r"echo 'This does nothing.'".into()),
                    true,
                ));
                t.add_step(
                    Step::new(Capturable::new("echo 0; sleep 3; echo 1".into()), true)
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&runner, &HashMap::new()).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::TimedOut,
                    output: vec![ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: r"echo 'This does nothing.'".into(),
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
                &std::env::current_dir().unwrap(),
                JudgerPrivateConfig {
                    test_root_dir: PathBuf::from(r"../golem/src"),
                    mapped_test_root_dir: PathBuf::from(r"/golem/src"),
                },
                JudgerPublicConfig {
                    time_limit: None,
                    memory_limit: None,
                    name: "golem_no_volume".into(),
                    test_groups: {
                        [(
                            "default".to_owned(),
                            vec![TestCaseDefinition {
                                name: "succ".into(),
                                should_fail: false,
                                has_out: true,
                            }],
                        )]
                        .iter()
                        .cloned()
                        .collect()
                    },
                    vars: [
                        ("$src", "py"),
                        ("$bin", "pyc"),
                        ("$stdin", "in"),
                        ("$stdout", "out"),
                    ]
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                    run: ["cat $stdin | python ./golem.py $bin"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),

                    mapped_dir: Bind {
                        from: PathBuf::from(r"../golem/src"),
                        to: PathBuf::from(r"../golem/src"),
                        readonly: false,
                    },
                    binds: None,
                    special_judge_script: None,
                },
                &JudgeTomlTestConfig {
                    // TODO: Refine interface
                    image: Image::Image { tag: "".into() },
                    build: None,
                    run: vec!["python ./golemc.py $src -o $bin".into()],
                },
                TestSuiteOptions {
                    tests: ["succ"].iter().map(|s| s.to_string()).collect(),
                    time_limit: None,
                    mem_limit: None,
                    build_image: true,
                    remove_image: true,
                },
            )
            .await?;

            let instance = bollard::Docker::connect_with_local_defaults().unwrap();
            ts.run(
                instance,
                std::env::current_dir().unwrap(),
                None,
                None,
                None,
                Default::default(),
            )
            .await?;
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
                &std::env::current_dir().unwrap(),
                JudgerPrivateConfig {
                    test_root_dir: PathBuf::from(r"../golem/src"),
                    mapped_test_root_dir: PathBuf::from(r"/golem/src"),
                },
                JudgerPublicConfig {
                    time_limit: None,
                    memory_limit: None,
                    name: "golem".into(),
                    test_groups: {
                        [(
                            "default".to_owned(),
                            vec![TestCaseDefinition {
                                name: "succ".into(),
                                should_fail: false,
                                has_out: true,
                            }],
                        )]
                        .iter()
                        .cloned()
                        .collect()
                    },
                    vars: [
                        ("$src", "py"),
                        ("$bin", "pyc"),
                        ("$stdin", "in"),
                        ("$stdout", "out"),
                    ] // public
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                    run: ["cat $stdin | python ./golem.py $bin"] // public
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),

                    mapped_dir: Bind {
                        from: PathBuf::from(r"../golem/src"),
                        to: PathBuf::from(r"/golem/src"),
                        readonly: false,
                    },
                    binds: Some(vec![]),
                    special_judge_script: None,
                },
                &JudgeTomlTestConfig {
                    // TODO: Refine interface
                    image: Image::Image { tag: "".into() },
                    build: None,
                    run: vec!["python ./golemc.py $src -o $bin".into()],
                },
                TestSuiteOptions {
                    tests: ["succ"].iter().map(|s| s.to_string()).collect(), // private
                    time_limit: None,                                        // private
                    mem_limit: None,                                         // private
                    build_image: true,                                       // private
                    remove_image: true,                                      // private
                },
            )
            .await?;

            let instance = bollard::Docker::connect_with_local_defaults().unwrap();
            ts.run(
                instance,
                std::env::current_dir().unwrap(),
                None,
                None,
                None,
                Default::default(),
            )
            .await?;
            Ok(())
        })
    }
}
