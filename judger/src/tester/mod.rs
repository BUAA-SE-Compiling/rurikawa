//! Everything around test suite interpretation and judging.
//!
//! This module is not responsible for any concrete judging implementation. See
//! [`crate::runner`] for detail on image builder and command runners.

use std::path::Path;

use bollard::models::{BuildInfo, Mount};
use bollard::Docker;
use tokio::sync::mpsc::UnboundedSender;

use crate::prelude::{CancellationTokenHandle, FlowSnake};
use crate::runner;
use crate::runner::exec::{Container, CreateContainerConfig, CreateContainerConfigBuilder};
use crate::runner::image::{build_image, BuildImageOptionsBuilder, BuildImageResult};
use crate::tester::model::BuildError;

use self::model::{Image, JobFailure, JudgeExecKind, JudgerPublicConfig};

pub mod model;
pub mod runner_plan;
pub mod spj;
pub mod utils;

/// Build the container used in the public config
pub async fn build_judger_container(
    docker: Docker,
    pub_cfg: &JudgerPublicConfig,
    base_path: &Path,
    guid: &str,
    cfg: CreateContainerConfig,
) -> anyhow::Result<Option<Container>> {
    tracing::info!(%guid, "Building judger container");

    match pub_cfg.exec_kind {
        JudgeExecKind::Legacy => {
            tracing::info!("Legacy judging, no container");
            Ok(None)
        }
        JudgeExecKind::Isolated => {
            make_isolated_judger_container(docker, pub_cfg, base_path, guid, cfg)
                .await
                .map(Some)
        }
    }
}

/// Build the container used in the public config, where it is guaranteed to be [`JudgeExecKind::Isolated`].
///
/// # Panics
///
/// This function asserts that `pub_cfg.exec_kind == JudgeExecKind::Isolated`.
async fn make_isolated_judger_container(
    docker: Docker,
    pub_cfg: &JudgerPublicConfig,
    base_path: &Path,
    guid: &str,
    cfg: CreateContainerConfig,
) -> anyhow::Result<Container> {
    debug_assert!(pub_cfg.exec_kind == JudgeExecKind::Isolated);
    if pub_cfg.exec_environment.is_none() {
        return Err(anyhow::Error::msg(
            "When `execKind` == isolated, an `execEnvironment` must be present",
        ));
    }

    let tag = format!("test-container-{}:{}", pub_cfg.name, guid);

    tracing::info!(%tag, "Looking for image for judger container");

    let image = match docker.inspect_image(&tag).await {
        Ok(image) => image,
        Err(bollard::errors::Error::DockerResponseNotFoundError { .. }) => {
            let exec_environment = pub_cfg.exec_environment.as_ref().unwrap();

            let opt = BuildImageOptionsBuilder::default()
                .base_path(base_path)
                .tag_as(tag.clone())
                .cancellation(cfg.cancellation.clone())
                .build()
                .expect("Failed to generate build options");
            let BuildImageResult {} =
                runner::image::build_image(docker.clone(), exec_environment, opt).await?;
            docker.inspect_image(&tag).await?
        }
        Err(e) => return Err(e.into()),
    };

    tracing::info!(%tag, %image.id, "Creating container from image");

    Container::create(docker, image.id, cfg)
        .await
        .map_err(|e| e.into())
}

pub async fn build_user_code_container(
    docker: Docker,
    image_name: &str,
    image: &Image,
    config_build_image_options: impl FnOnce(BuildImageOptionsBuilder) -> BuildImageOptionsBuilder,
    config_create_container_configs: impl FnOnce(
        CreateContainerConfigBuilder,
    ) -> CreateContainerConfigBuilder,
) -> Result<Container, BuildError> {
    tracing::info!(%image_name, "Building user code image");
    let cfg = config_build_image_options(BuildImageOptionsBuilder::default())
        .tag_as(image_name)
        .build()
        .expect("Failed to generate build options");

    let _image = build_image(docker.clone(), image, cfg).await?;

    tracing::info!(%image_name, "Creating container from user code image");

    let cfg = config_create_container_configs(CreateContainerConfigBuilder::default())
        .build()
        .expect("Failed to generate CreateContainerConfig");

    let container = Container::create(docker, image_name.to_owned(), cfg)
        .await
        .map_err(|e| BuildError::Internal(e.into()))?;

    Ok(container)
}
