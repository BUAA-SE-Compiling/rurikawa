//! Code for transforming test suite configs into something that [`crate::runner`]
//! can efficiently use.

use std::{collections::HashMap, path::Path, pin::Pin, sync::Arc, time::Duration};

use futures::{Sink, SinkExt};
use itertools::Itertools;
use path_slash::PathBufExt;

use crate::config::JudgeTomlTestConfig;
use crate::prelude::CancellationTokenHandle;
use crate::runner::{
    model::{CommandRunOptionsBuilder, ProcessOutput},
    CommandRunner,
};
use crate::{
    client::model::Job,
    runner::model::{ExecGroup, ExecStep, OutputComparisonSource, TestCase},
};

use super::model::{
    ExecError, ExecErrorKind, JobFailure, JudgeExecKind, JudgerPublicConfig, ShouldFailFailure,
    TestCaseDefinition,
};

/// A raw result that's been generated from running a test case.
pub struct RawTestCaseResult(
    pub String,
    pub Result<(), JobFailure>,
    pub Vec<ProcessOutput>,
);

type BoxSink<R, E> = Pin<Box<dyn Sink<R, Error = E> + Send>>;

/// Run all test cases for a certain job, and collect their results.
pub async fn run_job_test_cases<'a>(
    job: &'a Job,
    public_cfg: &'a JudgerPublicConfig,
    judge_toml: &'a JudgeTomlTestConfig,
    user_container: Arc<dyn CommandRunner>,
    judger_container: Option<Arc<dyn CommandRunner>>,
    mut raw_result_sink: BoxSink<RawTestCaseResult, ()>,
    test_suite_base_dir: &'a Path,
    cancel: CancellationTokenHandle,
) -> anyhow::Result<()> {
    tracing::info!(%job.id, "Planning to run job");

    // This index ensures all test cases specified in `job` are present, and also
    // provides a map between test names and cases.
    let public_cfg_verification_index = public_cfg
        .test_groups
        .iter()
        .map(|(_group, items)| items.iter())
        .flatten()
        .map(|case| (case.name.as_str(), case))
        .collect::<HashMap<_, _>>();

    let run_option = CommandRunOptionsBuilder::default()
        .cancel(cancel.clone())
        .timeout(public_cfg.time_limit.map(Duration::from_secs_f64))
        .build()
        .expect("Failed to build command run options");

    for case in job
        .tests
        .iter()
        .sorted()
        .dedup()
        .filter_map(|case| public_cfg_verification_index.get(case.as_str()))
    {
        tracing::debug!(job = %job.id, case = %case.name, "Running test case in job");
        let (runner_case, additional_flags) = generate_test_case(
            case,
            public_cfg,
            judge_toml,
            user_container.clone(),
            judger_container.clone(),
            test_suite_base_dir,
        );

        let (sink, mut recv) = tokio::sync::mpsc::channel(19);
        let output_collector = tokio::spawn(async move {
            let mut res = vec![];
            while let Some(v) = recv.recv().await {
                res.push(v)
            }
            res
        });

        let case_res = crate::runner::run_test_case(&runner_case, &run_option, sink).await?;
        let case_res = apply_additional_run_flags(case_res, additional_flags);
        let output = output_collector
            .await
            .expect("Unable to join output collection task. Anything went wrong?");

        let res = RawTestCaseResult(case.name.clone(), case_res, output);

        raw_result_sink
            .send(res)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send result across sink"))?;
    }

    Ok(())
}

pub fn apply_additional_run_flags(
    mut result: Result<(), JobFailure>,
    additional: AdditionalRunFlags,
) -> Result<(), JobFailure> {
    // apply `should_fail` flag, which transforms ReturnCodeCheckFailed into Ok
    if additional.should_fail {
        result = match result {
            Ok(()) => Err(JobFailure::ShouldFail(ShouldFailFailure)),
            Err(JobFailure::ExecError(ExecError {
                kind: ExecErrorKind::ReturnCodeCheckFailed,
                ..
            })) => Ok(()),
            other => other,
        }
    }

    result
}

/// Generate a test case from its definition and other configs
pub fn generate_test_case(
    test_case: &TestCaseDefinition,
    public_cfg: &JudgerPublicConfig,
    judge_toml: &JudgeTomlTestConfig,
    user_container: Arc<dyn CommandRunner>,
    judger_container: Option<Arc<dyn CommandRunner>>,
    test_suite_base_dir: &Path,
) -> (TestCase, AdditionalRunFlags) {
    debug_assert!(
        judger_container.is_some() == (public_cfg.exec_kind == JudgeExecKind::Isolated),
        "should be verified in previous steps"
    );

    let has_judger_container = public_cfg.exec_kind == JudgeExecKind::Isolated;
    // whether this test case should fail. Nah, `should_fail` flags are not
    // processed in the `runner` module anyway.
    let should_fail = test_case.should_fail;
    // whether this test case has output checking.
    // NOTE: `should_fail` implies `!has_out`.
    let has_out = test_case.has_out && !should_fail;

    let mut run_in_user_container: ExecGroup = ExecGroup {
        run_in: user_container,
        steps: vec![],
    };

    let mut run_in_judger_container: Option<ExecGroup> =
        judger_container.map(|container| ExecGroup {
            run_in: container,
            steps: vec![],
        });

    let mut env = Vec::new();
    for (src, tgt) in &public_cfg.vars {
        let src = src.strip_prefix('$').unwrap_or(src);
        let tgt = Path::new(&public_cfg.mapped_dir.to)
            .join(format!("{}.{}", test_case.name, tgt))
            .to_slash_lossy();
        env.push((src.into(), tgt.to_string()));
    }
    env.push(("CI".into(), "1".into()));
    env.push(("JUDGE".into(), "1".into()));

    let env_mounting_point = Arc::new(env);

    // add user commands
    for cmd in &judge_toml.run {
        let step = ExecStep {
            env: env_mounting_point.clone(),
            run: cmd.clone(),
            compare_output_with: None,
        };
        run_in_user_container.steps.push(step);
    }

    // add judge commands
    for cmd in &public_cfg.run {
        let step = ExecStep {
            env: env_mounting_point.clone(),
            run: cmd.clone(),
            compare_output_with: None,
        };
        if has_judger_container {
            run_in_judger_container.as_mut().unwrap().steps.push(step);
        } else {
            run_in_user_container.steps.push(step);
        }
    }

    if has_out && public_cfg.vars.contains_key("$stdout") {
        // enable output comparison
        let last_command = run_in_judger_container
            .as_mut()
            .and_then(|g| g.steps.last_mut())
            .or_else(|| run_in_user_container.steps.last_mut());

        if let Some(cmd) = last_command {
            cmd.compare_output_with = Some(OutputComparisonSource::File(test_suite_base_dir.join(
                public_cfg.mapped_dir.from.join(format!(
                    "{}.{}",
                    test_case.name,
                    public_cfg.vars.get("$stdout").expect("$stdout must exist")
                )),
            )))
        }
    }

    let mut test_case = TestCase {
        commands: vec![run_in_user_container],
    };
    if let Some(judger_container) = run_in_judger_container {
        test_case.commands.push(judger_container);
    }

    let additional_run_flags = AdditionalRunFlags { should_fail };

    (test_case, additional_run_flags)
}

#[derive(Debug)]
pub struct AdditionalRunFlags {
    /// Should this test case fail by design?
    ///
    /// It will be a failure if this field is `true` and all commands
    /// in this test case have a `0` exit value.
    pub should_fail: bool,
}
