use std::{sync::Arc, time::Duration};

use crate::{
    runner::{
        model::{
            CommandRunOptionsBuilder, CommandRunner, ExecGroup, ExecStep, OutputComparisonSource,
            TestCase,
        },
        run_test_case,
    },
    tester::model::JobFailure,
};

use super::util::MockRunner;

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

#[tokio::test]
async fn test_exec() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(0)
        .stdout("foo")
        .finish();

    let container = Arc::new(container);

    let (_env, test_case) = make_env_and_test_case(container.clone());

    let opt = CommandRunOptionsBuilder::default()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // 20 is enough to buffer all messages
    let (sink, mut iter) = tokio::sync::mpsc::channel(20);

    run_test_case(&test_case, &opt, sink)
        .await
        .expect("Error running docker")
        .expect("Job failed");

    while let Some(_res) = iter.recv().await {}
}

#[tokio::test]
async fn test_exec_compare_error() {
    let mut container = MockRunner::new();
    container
        .when("python ./golemc.py /src/succ.py -o /src/succ.pyc")
        .returns(0)
        .stdout("bar")
        .finish();

    let container = Arc::new(container);

    let (_env, test_case) = make_env_and_test_case(container.clone());

    let opt = CommandRunOptionsBuilder::default()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // 20 is enough to buffer all messages
    let (sink, mut iter) = tokio::sync::mpsc::channel(20);

    match run_test_case(&test_case, &opt, sink)
        .await
        .expect("Error running docker")
    {
        Ok(_) => panic!("The test should fail"),
        Err(JobFailure::OutputMismatch(_)) => {}
        Err(e) => panic!("The test should fail with output mismatch, got {:?}", e),
    };

    while let Some(_res) = iter.recv().await {}
}

#[tokio::test]
#[ignore]
async fn docker_integration_test() {
    // let docker = Docker::connect_with_local_defaults().expect("Failed to connect docker");

    // let image = Image::Dockerfile {
    //     path: ".".into(),
    //     file: None,
    // };
    // let image_name = "rurikawa/test_suite_basic_image";
    // let opt = BuildImageOptionsBuilder::default()
    //     .base_path("../golem/")
    //     .tag_as(image_name)
    //     .build()
    //     .unwrap();

    // let _ = crate::runner::image::build_image(docker.clone(), &image, opt)
    //     .await
    //     .expect("Failed to build image");

    // let opt = CreateContainerConfigBuilder::default().build().unwrap();

    // let collector = AsyncTeardownCollector::new();
    // // let test_container = Arc::new()

    // let container = Arc::new(
    //     Container::create(docker, image_name.into(), opt)
    //         .await
    //         .expect("Failed to build container"),
    // );

    // let env: Arc<Vec<(String, String)>> = Arc::new(
    //     [
    //         ("CI", "true"),
    //         ("src", "/src/succ.py"),
    //         ("bin", "/src/succ.pyc"),
    //         ("stdin", "/src/succ.in"),
    //     ]
    //     .iter()
    //     .map(|&(x, y)| (x.into(), y.into()))
    //     .collect(),
    // );

    // let test_case = TestCase {
    //     commands: vec![ExecGroup {
    //         run_in: container,
    //         steps: vec![ExecStep {
    //             env: env.clone(),
    //             run: "python ./golemc.py $src -o $bin".into(),
    //             compare_output_with: None,
    //         }],
    //     }],
    // };

    // let opt = CommandRunOptionsBuilder::default()
    //     .timeout(Duration::from_secs(10))
    //     .build()
    //     .unwrap();

    // // 20 is enough to buffer all messages
    // let (sink, mut iter) = tokio::sync::mpsc::channel(20);

    // run_test_case(&test_case, &opt, sink)
    //     .await
    //     .expect("Error running docker")
    //     .expect("Job failed");

    // while let Some(res) = iter.recv().await {}

    // collector.teardown_all().await;
}
