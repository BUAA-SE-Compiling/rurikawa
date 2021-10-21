use std::{collections::HashMap, sync::Arc, time::Duration};

use bollard::Docker;
use bytes::Bytes;
use tokio_test;

use crate::{
    config::Image,
    runner::{
        exec::{Container, CreateContainerConfigBuilder},
        image::BuildImageOptionsBuilder,
        model::{CommandRunOptionsBuilder, ExecGroup, ExecStep, TestCase},
        run_test_case,
    },
    tester::runner_plan::run_job_test_cases,
    util::AsyncTeardownCollector,
};

use super::util::tar_with_files;

#[test]
fn test_suite_basic() {
    tokio_test::block_on(async {
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
    })
}
