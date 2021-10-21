//! Code for transforming test suite configs into something that [`crate::runner`]
//! can efficiently use.

use std::{collections::HashMap, sync::Arc, time::Duration};

use itertools::Itertools;

use crate::runner::{
    model::{CommandRunOptionsBuilder, ProcessOutput},
    CommandRunner,
};
use crate::{
    client::model::Job,
    runner::model::{ExecGroup, ExecStep, OutputComparisonSource, TestCase},
};
use crate::{config::JudgeTomlTestConfig, tester::model::canonical_join};

use super::model::{JobFailure, JudgeExecKind, JudgerPublicConfig, TestCaseDefinition};

/// Run all test cases for a certain job, and collect their results.
pub async fn run_job_test_cases<'a>(
    job: &'a Job,
    public_cfg: &'a JudgerPublicConfig,
    judge_toml: &'a JudgeTomlTestConfig,
    user_container: Arc<dyn CommandRunner>,
    judger_container: Option<Arc<dyn CommandRunner>>,
) -> anyhow::Result<Vec<(String, Result<(), JobFailure>, Vec<ProcessOutput>)>> {
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
        .timeout(public_cfg.time_limit.map(Duration::from_secs_f64))
        .build()
        .expect("Failed to build command run options");

    let mut run_result = vec![];

    for case in job
        .tests
        .iter()
        .sorted()
        .dedup()
        .filter_map(|case| public_cfg_verification_index.get(case.as_str()))
    {
        tracing::trace!(job = %job.id, case = %case.name, "Running test case in job");
        let runner_case = generate_test_case(
            case,
            public_cfg,
            judge_toml,
            user_container.clone(),
            judger_container.clone(),
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
        let output = output_collector
            .await
            .expect("Unable to join output collection task. Anything went wrong?");

        run_result.push((case.name.clone(), case_res, output));
        // TODO: do something with output
    }

    Ok(run_result)
}

/// Generate a test case from its definition and other configs
pub fn generate_test_case(
    test_case: &TestCaseDefinition,
    public_cfg: &JudgerPublicConfig,
    judge_toml: &JudgeTomlTestConfig,
    user_container: Arc<dyn CommandRunner>,
    judger_container: Option<Arc<dyn CommandRunner>>,
) -> TestCase {
    debug_assert!(
        judger_container.is_some() == (public_cfg.exec_kind == JudgeExecKind::Isolated),
        "should be verified in previous steps"
    );

    let has_judger_container = public_cfg.exec_kind == JudgeExecKind::Isolated;

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
        let tgt = canonical_join(
            &public_cfg.mapped_dir.to,
            format!("{}.{}", test_case.name, tgt),
        );
        env.push((src.into(), tgt.display().to_string()));
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

    if test_case.has_out && public_cfg.vars.contains_key("$stdout") {
        // enable output comparison
        let last_command = run_in_judger_container
            .as_mut()
            .and_then(|g| g.steps.last_mut())
            .or_else(|| run_in_user_container.steps.last_mut());

        if let Some(cmd) = last_command {
            cmd.compare_output_with = Some(OutputComparisonSource::File(
                public_cfg.mapped_dir.from.join(format!(
                    "{}.{}",
                    test_case.name,
                    public_cfg.vars.get("$stdout").expect("$stdout must exist")
                )),
            ))
        }
    }

    let mut test_case = TestCase {
        commands: vec![run_in_user_container],
    };
    if let Some(judger_container) = run_in_judger_container {
        test_case.commands.push(judger_container);
    }

    test_case
}
