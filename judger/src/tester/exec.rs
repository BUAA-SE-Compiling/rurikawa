use super::{
    model::*,
    runner::{CommandRunner, DockerCommandRunner, DockerCommandRunnerOptions},
    spj::{self, SpjEnvironment},
    utils::diff,
    BuildError, ExecError, ExecErrorKind, JobFailure, OutputMismatch, ProcessInfo,
    ShouldFailFailure,
};
use crate::{
    client::model::{upload_test_result, ResultUploadConfig, TestResult, TestResultKind},
    config::JudgeTomlTestConfig,
    prelude::*,
};
use anyhow::Result;
use bollard::models::{BuildInfo, Mount};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use path_slash::PathBufExt;
use std::{collections::HashMap, io, path::Path, path::PathBuf, sync::Arc, time};
use tokio::{io::AsyncReadExt, sync::mpsc::UnboundedSender};
use tokio_util::compat::*;

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

/// A `Capturable` represents a pending subprocess call using a [`CommandRunner`]
/// inside [`sh` (Bourne shell)][sh] or any compatible shell. The command inside
/// `Capturable` MUST be a valid Bourne shell commandline string, capable of being
/// called using `sh -c '...'`.
///
/// [sh]: https://en.wikipedia.org/wiki/Bourne_shell
pub struct Capturable(String);

impl Capturable {
    pub fn new(cmd: String) -> Self {
        Capturable(cmd)
    }

    /// Run the command represented by `self` with the given `runner`, with
    /// `variables` representing commandline variables feeding to `sh` to
    /// replace corresponding `$...` inside the commandline.
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
            tokio::time::timeout(timeout, self.cmd.capture(runner, variables))
                .await
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("Popen capture timed out at {}s", timeout.as_secs_f64()),
                    )
                })?
                .map(|mut i| {
                    i.is_user_command = is_user_command;
                    i
                })
        } else {
            self.cmd
                .capture(runner, variables)
                .await
                .map(|i| ProcessInfo {
                    is_user_command,
                    ..i
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
    /// Run this specific test. Returns the score of this test (`1` when scoring mode is off).
    pub async fn run<R>(
        self,
        runner: &R,
        variables: &HashMap<String, String>,
        spj: Option<&mut SpjEnvironment>,
    ) -> Result<f64, JobFailure>
    where
        R: CommandRunner + Send,
    {
        let spj_enabled = spj.as_ref().map_or(false, |x| x.features().case());
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
                    } else if spj_enabled {
                        // continue
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

            if i == steps_len - 1 && !spj_enabled {
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

        if spj_enabled {
            // do special judging
            // spj_enabled would only be true when spj is Some(_)
            let spj = spj.unwrap();
            let judge_result = spj
                .spj_case_judge(&output)
                .await
                .map_err(JobFailure::internal_err_from)?;

            if judge_result.accepted {
                Ok(judge_result.score.unwrap_or(1.0))
            } else {
                Err(JobFailure::SpjWrongAnswer(super::SpjFailure {
                    reason: judge_result.reason,
                    output,
                }))
            }
        } else if self.should_fail && !test_failed {
            // Tests that _should_ fail but did not should return error here
            return Err(JobFailure::ShouldFail(ShouldFailFailure { output }));
        } else {
            Ok(1.0)
        }
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
            Image::Dockerfile { path, .. } => {
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
        cancel: CancellationTokenHandle,
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
                let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
                let read_codec = tokio_util::codec::BytesCodec::new();
                let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);
                let task = async move {
                    let mut tar = async_tar::Builder::new(futures::io::BufWriter::new(
                        pipe_recv.compat_write(),
                    ));
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
                                .map(|x| x.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "Dockerfile".into()),
                            t: tag.into(),
                            rm: true,
                            forcerm: true,

                            // TODO: we currently limit the builder to only use 1/2 cpu
                            // i.e. <= 50ms every 100ns
                            cpuperiod: Some(100_000),
                            cpuquota: Some(50_000),
                            buildargs: [("CI", "true")]
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                            ..Default::default()
                        },
                        None,
                        // Freeze `path` as a tar archive.
                        Some(hyper::Body::wrap_stream(frame)),
                    )
                    .map(|x| match x {
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

        // Get command steps
        let mut raw_steps = job_cfg
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
            .collect::<Vec<_>>();

        // Initialize special judge
        let spj = if let Some(script) = &public_cfg.special_judge_script {
            let script_path = base_dir.join(script);
            let mut spj = spj::make_spj(&script_path).await?;

            // Do special judge initialization
            spj.with_console_env("todo".into())?;
            spj.with_readfile(base_dir.to_owned())?;
            spj.spawn_futures().await;
            if spj.features().global_init() {
                spj.spj_global_init(&public_cfg).await?;
            }
            if spj.features().transform_exec() {
                raw_steps = spj.spj_map_exec(&raw_steps).await?;
            }
            Some(spj)
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
        cancellation_token: CancellationTokenHandle,
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

            if let Some(spj) = &mut self.spj_env {
                if spj.features().case_init() {
                    log::trace!("{:08x}: spj init {}", rnd_id, case.name);
                    spj.spj_case_init(case, &replacer).await?;
                }
            }

            log::trace!("{:08x}: created test: {}", rnd_id, case.name);

            let res = t
                .run(&runner, &replacer, self.spj_env.as_mut())
                .with_cancel(cancellation_token.clone())
                .await
                .ok_or(JobFailure::Cancelled)
                .and_then(|x| x);
            log::trace!("{:08x}: runned: {}", rnd_id, case.name);

            let (mut res, cache) = TestResult::from_result(res, case.base_score);
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

/// Create a test case out of various configs.
///
/// This function is extracted from TestSuite::Run.
/// TODO: Refactor this function.
async fn create_test_case(
    public_cfg: &JudgerPublicConfig,
    test_root: &Path,
    container_test_root: &Path,
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
                let p = match var.as_ref() {
                    "$stdout" => test_root,
                    _ => container_test_root,
                };
                p.join(format!("{}.{}", name, ext)).to_slash_lossy()
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
        base_score: case.base_score,
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

mod test_suite;
mod tests;
