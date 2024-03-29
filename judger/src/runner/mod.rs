//! Concrete implementation on running tests.
//!
//! This module is not responsible for interpreting test suites. See [`crate::tester`]
//! for corresponding code.

use std::borrow::Cow;

use once_cell::sync::Lazy;
use tokio::sync::mpsc::UnboundedSender;

use crate::prelude::CancelFutureExt;
use crate::tester::{
    model::{ExecError, ExecErrorKind, JobFailure},
    utils::{diff, strsignal},
};

use self::model::{ExitStatus, OutputComparisonSource, ProcessOutput};

pub mod exec;
pub mod image;
pub mod model;
mod util;
pub mod volume;

pub use model::CommandRunner;

type TestOutput = anyhow::Result<Result<(), JobFailure>>;

/// Run a testcase.
///
/// # Error Handling
///
/// Returning `Ok(_)` means this case doesn't encounter any internal error, the
/// result of the test case is wrapped in another `Result<(), JobFailure>`
/// inside the outer error.
pub async fn run_test_case(
    exec: &model::TestCase,
    opt: &model::CommandRunOptions,
    sink: UnboundedSender<ProcessOutput>,
) -> TestOutput {
    tracing::debug!("Starting new test case");
    for group in &exec.commands {
        match run_exec_group(group, opt, sink.clone()).await {
            Ok(Ok(_)) => {}
            e => return e,
        };
    }
    Ok(Ok(()))
}

/// Run an execution group inside a test case.
///
/// # Error Handling
///
/// Returning `Ok(_)` means this case doesn't encounter any internal error, the
/// result of the test case is wrapped in another `Result<(), JobFailure>`
/// inside the outer error.
pub async fn run_exec_group(
    group: &model::ExecGroup,
    opt: &model::CommandRunOptions,
    sink: UnboundedSender<ProcessOutput>,
) -> TestOutput {
    tracing::debug!(run_in = %group.run_in.name(), "Starting exec group");
    for exec in &group.steps {
        tracing::debug!(command = %exec.run, "Running command");
        let run_res = match group
            .run_in
            .run(
                &exec.run,
                &mut exec.env.iter().map(|(k, v)| (k.as_ref(), v.as_ref())),
                opt,
            )
            .with_cancel(opt.cancel.cancelled())
            .await
        {
            None => return Ok(Err(JobFailure::Cancelled)),

            Some(Ok(o)) => o,
            Some(Err(e)) => {
                return Err(e);
            }
        };

        let ret_code = run_res.ret_code.clone();
        if ret_code != ExitStatus::ReturnCode(0) {
            tracing::debug!(?ret_code, "Return code check failed");
            sink.send(run_res)?;

            if ret_code == ExitStatus::Timeout {
                return Ok(Err(JobFailure::ExecError(ExecError {
                    command: exec.run.clone(),
                    kind: ExecErrorKind::TimedOut,
                })));
            } else if let ExitStatus::Signal(sig) = ret_code {
                return Ok(Err(JobFailure::ExecError(ExecError {
                    command: exec.run.clone(),
                    kind: ExecErrorKind::RuntimeError(strsignal(sig as i32).into_owned()),
                })));
            } else {
                return Ok(Err(JobFailure::ExecError(ExecError {
                    command: exec.run.clone(),
                    kind: ExecErrorKind::ReturnCodeCheckFailed,
                })));
            }
        }

        if let Some(cmp_source) = &exec.compare_output_with {
            tracing::debug!("Output comparison failed");
            let output_res = match verify_output(cmp_source, &run_res).await {
                Ok(o) => o,
                Err(e) => {
                    // workaround before async_drop stabilizes
                    sink.send(run_res)?;
                    return Err(e);
                }
            };
            if let Some(diff) = output_res {
                sink.send(run_res)?;
                return Ok(Err(JobFailure::OutputMismatch(diff)));
            }
        }

        sink.send(run_res)?;
    }
    Ok(Ok(()))
}

/// Verify a process's output. Returns `Ok(Some(diff_string))` if they don't
/// match, `Ok(None)` if they match, and `Err(_)` if anything else happens.
pub async fn verify_output(
    cmp_source: &OutputComparisonSource,
    output: &ProcessOutput,
) -> anyhow::Result<Option<String>> {
    let expected: Cow<str> = match cmp_source {
        OutputComparisonSource::File(path) => tokio::fs::read_to_string(path).await?.into(),
        OutputComparisonSource::InMemory(s) => s.into(),
    };

    let expected = EOF_PATTERN.replace_all(expected.trim(), "\n");
    let stdout = EOF_PATTERN.replace_all(output.stdout.trim(), "\n");

    let diff = diff(&stdout, &expected);
    if diff.0 {
        Ok(Some(diff.1))
    } else {
        Ok(None)
    }
}

static EOF_PATTERN: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\r?\n").unwrap());
