//! Tests to verify that [`crate::runner`] functions behave correctly.

use std::{sync::Arc, time::Duration};

use crate::{
    runner::{
        model::{
            CommandRunOptionsBuilder, CommandRunner, ExecGroup, ExecStep, ExitStatus,
            OutputComparisonSource, TestCase,
        },
        run_test_case,
    },
    tester::model::{ExecError, ExecErrorKind, JobFailure},
};

use super::util::MockRunner;
use test_env_log::test;

fn make_env_and_test_case(
    container: Arc<dyn CommandRunner>,
) -> (Arc<Vec<(String, String)>>, TestCase) {
    let env: Arc<Vec<(String, String)>> = Arc::new(
        [
            ("CI", "true"),
            ("src", "/src/succ.py"),
            ("bin", "/src/succ.pyc"),
            ("stdin", "/src/succ.in"),
        ]
        .iter()
        .map(|&(x, y)| (x.into(), y.into()))
        .collect(),
    );

    let test_case = TestCase {
        commands: vec![ExecGroup {
            run_in: container,
            steps: vec![ExecStep {
                env: env.clone(),
                run: "python ./golemc.py $src -o $bin".into(),
                compare_output_with: Some(OutputComparisonSource::InMemory("foo".into())),
            }],
        }],
    };
    (env, test_case)
}

async fn run_simple_test_with_mock_runner(runner: MockRunner) -> Result<(), JobFailure> {
    let runner = Arc::new(runner);

    let (_env, test_case) = make_env_and_test_case(runner.clone());

    // 20 is enough to buffer all messages
    let (sink, _ch) = tokio::sync::mpsc::unbounded_channel();

    run_test_case(
        &test_case,
        &CommandRunOptionsBuilder::default()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
        sink,
    )
    .await
    .expect("Error running docker")
}

#[test(tokio::test)]
async fn test_exec() {
    let mut runner = MockRunner::new();
    runner
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(0)
        .stdout("foo")
        .finish();

    run_simple_test_with_mock_runner(runner)
        .await
        .expect("Job failed");
}

#[test(tokio::test)]
async fn test_exec_compare_error() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(0)
        .stdout("bar")
        .finish();

    match run_simple_test_with_mock_runner(container).await {
        Ok(_) => panic!("The test should fail"),
        Err(JobFailure::OutputMismatch(_)) => {}
        Err(e) => panic!("The test should fail with output mismatch, got {:?}", e),
    };
}

#[test(tokio::test)]
async fn test_exec_pipeline_error() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(123)
        .stdout("bar")
        .finish();

    match run_simple_test_with_mock_runner(container).await {
        Ok(_) => panic!("The test should fail"),
        Err(JobFailure::ExecError(ExecError {
            kind: ExecErrorKind::ReturnCodeCheckFailed,
            ..
        })) => {}
        Err(e) => panic!(
            "The test should fail with return code mismatch, got {:?}",
            e
        ),
    };
}

#[test(tokio::test)]
async fn test_exec_runtime_error() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(ExitStatus::Signal(11))
        .stdout("bar")
        .finish();

    match run_simple_test_with_mock_runner(container).await {
        Ok(_) => panic!("The test should fail"),
        Err(JobFailure::ExecError(ExecError {
            kind: ExecErrorKind::RuntimeError(_),
            ..
        })) => {}
        Err(e) => panic!("The test should fail with runtime error, got {:?}", e),
    };
}

#[test(tokio::test)]
async fn test_exec_timeout_error() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(ExitStatus::Timeout)
        .stdout("bar")
        .finish();

    match run_simple_test_with_mock_runner(container).await {
        Ok(_) => panic!("The test should fail"),
        Err(JobFailure::ExecError(ExecError {
            kind: ExecErrorKind::TimedOut,
            ..
        })) => {}
        Err(e) => panic!("The test should fail with timeout, got {:?}", e),
    };
}
