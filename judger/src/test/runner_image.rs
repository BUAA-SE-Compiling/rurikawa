//! Tests to verify that functions related to docker images behave correctly.
//!
//! Tests in this module should be ignored by default, since test environments may
//! not have docker, or the docker instance may not be exposed in the default
//! path.
//!
use bollard::Docker;
use test_env_log::test;

use crate::config::Image;
use crate::runner::exec::{Container, CreateContainerConfig};
use crate::runner::image::BuildImageOptionsBuilder;

use super::util::project_root_dir;

#[test(tokio::test)]
#[ignore]
async fn test_docker_image_building() {
    let (docker, image_name) = build_golem_image().await;

    let _ = docker.remove_image(image_name, None, None).await;
}

#[test(tokio::test)]
#[ignore]
async fn test_docker_container_creation() {
    let (docker, image_name) = build_golem_image().await;

    let cfg = CreateContainerConfig::builder()
        .build()
        .expect("Failed to build create container config");

    let mut container = Container::create(docker.clone(), image_name.to_string(), cfg)
        .await
        .expect("Failed to build container");

    container
        .remove()
        .await
        .expect("Failed to remove container");

    let _ = docker.remove_image(image_name, None, None).await;
}

async fn build_golem_image() -> (Docker, &'static str) {
    let docker = Docker::connect_with_local_defaults().expect("Failed to connect docker");
    let image = Image::Dockerfile {
        path: ".".into(),
        file: None,
    };
    let image_name = "rurikawa/test_suite_basic_image";
    let opt = BuildImageOptionsBuilder::default()
        .base_path(project_root_dir().join("../golem"))
        .tag_as(image_name)
        .build()
        .unwrap();
    let _ = crate::runner::image::build_image(docker.clone(), &image, opt)
        .await
        .expect("Failed to build image");
    (docker, image_name)
}
