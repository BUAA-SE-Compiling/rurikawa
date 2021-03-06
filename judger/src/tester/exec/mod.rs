mod test_suite;
mod tests;

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
use futures::prelude::*;
use itertools::Itertools;
use once_cell::sync::Lazy;
use path_slash::PathBufExt;
use std::{collections::HashMap, io, path::Path, path::PathBuf, sync::Arc, time};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    sync::mpsc::UnboundedSender,
};
use tokio_stream::wrappers::LinesStream;

use super::utils::strsignal;

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
/// inside [`sh` (Bourne shell)][sh] or any compatible shell.
///
/// # Attention
///
/// The command inside a [`Capturable`] _must_ be a valid [`sh`][sh] command,
/// capable of being called using `sh -c '...'`.
///
/// [sh]: https://en.wikipedia.org/wiki/Bourne_shell
pub struct Capturable(String);

impl Capturable {
    /// Create a new [`Captureable`] instance out of a command.
    ///
    /// # Arguments
    /// * `cmd` - The command to be run. It _must_ be a valid [`sh` (Bourne shell)][sh] command.
    ///
    /// [sh]: https://en.wikipedia.org/wiki/Bourne_shell
    pub fn new(cmd: impl AsRef<str>) -> Self {
        Capturable(cmd.as_ref().to_owned())
    }

    /// Run the command with the given `runner`.
    ///
    /// # Arguments
    ///
    /// * `runner` - The [`CommandRunner`] instance to be used when running the command.
    /// * `variables` - The `$...` variable bindings to be fed to `sh` when building the command.
    async fn capture(
        self,
        runner: &(impl CommandRunner + Send),
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo> {
        runner.run(&self.0, variables).await
    }
}

/// One step in a [`Test`].
pub struct Step {
    /// The command to be executed.
    pub cmd: Capturable,

    /// If the command is created by a user, rather than the admin.
    pub is_user_command: bool,

    /// The timeout of the command's execution.
    pub timeout: Option<time::Duration>,
}

impl Step {
    /// Make a new [`Step`] with no `timeout`.
    pub fn new(cmd: Capturable, is_user_command: bool) -> Self {
        Step {
            cmd,
            is_user_command,
            timeout: None,
        }
    }

    /// Set `timeout` for a [`Step`].
    pub fn set_timeout(mut self, timeout: time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Make a new [`Step`] with a `timeout`.
    pub fn with_timeout(
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

    /// Run the [`Step`] and collect its output info within the given `timeout`.
    ///
    /// # Arguments
    ///
    /// * `runner` - The [`CommandRunner`] instance to be used when running the [`Step`].
    /// * `variables` - The `$...` variable bindings to be fed to `sh` when building the [`Step`].
    pub async fn capture(
        self,
        runner: &(impl CommandRunner + Send),
        variables: &HashMap<String, String>,
    ) -> PopenResult<ProcessInfo> {
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
        } else {
            self.cmd.capture(runner, variables).await
        }
        .map(|i| ProcessInfo {
            is_user_command,
            ..i
        })
    }
}

static EOF_PATTERN: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\r?\n").unwrap());

/// A particular [`Test`] consisting of multiple [`Step`]s.
///
/// An `stdout` match test against `expected` is performed at the last [`Step`].
#[derive(Default)]
pub struct Test {
    /// The different [`Step`]s in this [`Test`].
    steps: Vec<Step>,

    /// The expected `stdout` content.
    expected: Option<String>,

    /// If this [`Test`] is _intended_ to fail.
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

    /// Run this specific [`Test`], and return a score (`1.0` when scoring mode is off).
    ///
    /// # Arguments
    ///
    /// * `runner` - The [`CommandRunner`] instance to be used.
    /// * `variables` - The `$...` variable bindings to be fed to `sh` when running this [`Test`].
    /// * `spj` - The special judge environment ([`SpjEnvironment`]) to be used.
    // ? Should `runner` be mutable?
    pub async fn run(
        self,
        runner: &(impl CommandRunner + Send),
        variables: &HashMap<String, String>,
        spj: Option<&mut SpjEnvironment>,
    ) -> Result<f64, JobFailure> {
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

            // Handle non-zero return code.
            #[allow(clippy::comparison_chain)]
            {
                let code = info.ret_code;
                if code > 0 {
                    if self.should_fail {
                        // Bail out of test, but it's totally fine.
                        test_failed = true;
                        break;
                    } else if spj_enabled {
                        // Ignore and continue with the rest.
                    } else {
                        return Err(JobFailure::ExecError(ExecError {
                            stage: i,
                            kind: ExecErrorKind::ReturnCodeCheckFailed,
                            output,
                        }));
                    }
                } else if code < 0 {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::RuntimeError(strsignal(-code).map_or_else(
                            || format!("Runtime Error: signal {}", -code),
                            |x| format!("Runtime Error: {} (signal {})", x, -code),
                        )),
                        output,
                    }));
                }
            }

            // Special case for the final step.
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

        // Handle special judge scoring, return the final result.
        // If special judge system is off, then the default return value should be `Ok(1.0)`.
        // TODO: Make `1.0` a variable.
        if spj_enabled {
            // Unwrapping is safe here: `spj_enabled` would only be true when spj is Some(_).
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
                    diff: judge_result.diff,
                    output,
                }))
            }
        } else if self.should_fail && !test_failed {
            // Tests that _should_ fail but didn't are considered malfunctioning.
            Err(JobFailure::ShouldFail(ShouldFailFailure { output }))
        } else {
            Ok(1.0)
        }
    }
}

pub type BuildResultChannel = UnboundedSender<BuildInfo>;

impl Image {
    pub fn set_dockerfile_tag(&mut self, new_tag: String) -> &mut Self {
        if let Image::Dockerfile { tag, .. } = self {
            *tag = new_tag;
        }
        self
    }

    pub fn tag(&self) -> String {
        match &self {
            Image::Prebuilt { tag, .. } => tag.to_owned(),
            Image::Dockerfile { tag, .. } => tag.to_owned(),
        }
    }

    /// Replace the relative `path` with respect to `base_dir` in [`Image::Dockerfile`] with the absolute one.
    pub fn canonicalize(&mut self, base_dir: PathBuf) -> &mut Self {
        if let Image::Dockerfile { path, .. } = self {
            if !path.is_absolute() {
                let mut path_base = base_dir;
                path_base.push(&path);
                *path = path_base;
            }
        }
        self
    }

    /// Build (or pull) the [`Image`] to make it usable in Docker.
    pub async fn build(
        &self,
        instance: bollard::Docker,
        partial_result_channel: Option<BuildResultChannel>,
        cancel: CancellationTokenHandle,
        network: Option<&str>,
        cpu_shares: Option<f64>,
    ) -> Result<(), BuildError> {
        match &self {
            Image::Prebuilt { tag } => instance
                .create_image(
                    Some(bollard::image::CreateImageOptions {
                        from_image: tag.to_owned(),
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .try_collect::<Vec<_>>()
                .map_ok(drop)
                .map_err(|e| {
                    BuildError::ImagePullFailure(format!("Failed to pull image `{}`: {}", tag, e))
                })
                .with_cancel(cancel)
                .await
                .ok_or(BuildError::Cancelled)?,

            Image::Dockerfile { tag, path, file } => {
                // We set the CPU quota here by using a period of 100ms
                let cpuquota = cpu_shares.map(|x| (x * 100_000f64).floor() as u64);
                let cpuperiod = cpuquota.is_some().then(|| 100_000);

                let ignore = ignore::gitignore::Gitignore::empty();

                // Launch a task for archiving.
                let (tar_stream, archiving) = crate::util::tar::pack_as_tar(&path, ignore)
                    .map_err(|e| BuildError::FileTransferError(e.to_string()))?;

                instance
                    .build_image(
                        bollard::image::BuildImageOptions {
                            dockerfile: file
                                .as_ref()
                                .map(|x| x.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "Dockerfile".into()),
                            t: tag.into(),
                            rm: true,
                            forcerm: true,

                            networkmode: network.unwrap_or("none").into(),

                            cpuperiod,
                            cpuquota,
                            buildargs: [("CI", "true")]
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                            ..Default::default()
                        },
                        None,
                        // Freeze `path` as a tar archive.
                        Some(hyper::Body::wrap_stream(tar_stream)),
                    )
                    .map_err(|e| BuildError::Internal(e.to_string()))
                    .try_for_each(|info| async {
                        if let Some(e) = info.error {
                            return Err(BuildError::BuildError {
                                error: e,
                                detail: info.error_detail,
                            });
                        }
                        if let Some(ch) = partial_result_channel.as_ref() {
                            let _ = ch.send(info);
                        }
                        Ok(())
                    })
                    .with_cancel(cancel.clone())
                    .await
                    .ok_or(BuildError::Cancelled)??;

                archiving
                    .await
                    .map_err(|e| BuildError::Internal(e.to_string()))?
                    .map_err(|e: io::Error| BuildError::FileTransferError(e.to_string()))?;

                Ok(())
            }
        }
    }

    /// Remove the Image when finished.
    ///
    /// # Attention
    /// This action must be done _after_ removing related containers.
    pub async fn remove_image(self, instance: bollard::Docker) -> Result<()> {
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

/// A suite of [`TestCase`]s to be run.
///
/// Attention: a [`TestSuite`] instance should NOT be constructed manually.
/// Please use `TestSuite::from_config`, for example.
pub struct TestSuite {
    /// An unique ID of this test suite
    pub id: String,

    /// The collection of [`TestCase`]s in this [`TestSuite`].
    pub test_cases: Vec<TestCase>,

    /// The [`Image`] which contains the compiler to be tested.
    image: Option<Image>,

    /// The volumes [`Mount`] bindings to be used in this [`TestCase`].
    pub binds: Option<Vec<Mount>>,

    ///`(source, dest)` pairs of data to be copied into the container.
    pub copies: Option<Vec<(String, String)>>,

    /// Ignored file pattern when copying data into the container
    pub copy_ignore: Vec<String>,

    /// Initialization options for [`TestSuite`].
    pub options: TestSuiteOptions,

    /// The collection of commands to execute within each test case.
    pub exec: Vec<RawStep>,

    /// Variables to be expanded at testing.
    ///
    /// Variables in this field are in the form of `{"$var": "dest"}`, which for example then
    /// expands `$var` inside test case `123` to `123.dest`.
    pub vars: HashMap<String, String>,

    /// Root folder of the [`TestSuite`] inside **this** machine.
    pub test_root: PathBuf,

    /// Root folder of the [`TestSuite`] inside **container**.
    pub container_test_root: PathBuf,

    /// Special Judger exectution environment used in this [`TestSuite`].
    spj_env: Option<spj::SpjEnvironment>,

    /// Network options
    network: NetworkOptions,
}

impl TestSuite {
    /// Push a [`TestCase`] to the current [`TestSuite`].
    pub fn add_case(&mut self, case: TestCase) {
        self.test_cases.push(case)
    }

    /// Build the [`TestSuite`] from given configurations.
    pub async fn from_config(
        id: String,
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

        let test_cases = futures::stream::iter(options.tests.clone().drain(..))
            .map(|name| {
                let case = index.get(&name).unwrap();
                create_test_case(&public_cfg, &test_root, &container_test_root, case, name)
            })
            .buffer_unordered(16)
            .try_collect::<Vec<_>>()
            .await?;

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
            .collect_vec();

        // Get ignored pattern.
        let copy_ignore = if let Some(file) = &public_cfg.test_ignore {
            let file = tokio::fs::File::open(file).await?;
            LinesStream::new(BufReader::new(file).lines())
                .try_collect()
                .await?
        } else {
            vec![]
        };

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
            id,
            image: Some(image),
            test_cases,
            options,
            exec: raw_steps,
            vars: public_cfg.vars,
            binds: public_cfg.binds.map(|bs| {
                bs.iter()
                    .map(|b| {
                        let mut b = b.clone();
                        b.canonicalize(base_dir);
                        b.to_mount()
                    })
                    .collect()
            }),
            copies: Some(vec![(
                canonical_join(base_dir, &public_cfg.mapped_dir.from).to_slash_lossy(),
                public_cfg.mapped_dir.to.to_slash_lossy(),
            )]),
            copy_ignore,
            spj_env: spj,
            test_root,
            container_test_root,
            network: public_cfg.network,
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
        let tag = image.tag();
        image
            .canonicalize(base_dir)
            .set_dockerfile_tag(format!("{}_{:08x}", tag, rnd_id));
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
                    network_options: self.network.clone(),
                    ..Default::default()
                }
            },
            build_result_channel,
        )
        .await?;

        // NOTE: DO NOT USE `?` OPERATOR AFTERWARDS, OR ELSE THE RUNNER CANNOT
        // BE DECONSTRUCTED PROPERLY!

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
                t.add_step(Step::with_timeout(
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
                .unwrap_or(Err(JobFailure::Cancelled));
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
    pub_cfg
        .test_groups
        .values()
        .flatten()
        .map(|test| (test.name.clone(), test))
        .collect()
}
