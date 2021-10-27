use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    config::Image,
    prelude::{CancelFutureExt, CancellationTokenHandle},
    runner::util::is_recoverable_error,
    tester::model::{canonical_join, BuildError},
    util::tar::pack_as_tar,
};

use bollard::{image::CreateImageOptions, models::BuildInfo, Docker};
use derive_builder::Builder;
use futures::{future::FusedFuture, FutureExt};
use hyper::Body;
use ignore::gitignore::Gitignore;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;

#[derive(Builder, Debug)]
#[builder(setter(into), pattern = "owned")]
pub struct BuildImageOptions {
    /// The base path of
    base_path: PathBuf,

    /// The tag of this image. Please select a tag that's unlikely to be used
    /// by other processes, e.g. with namespace and a UUID
    tag_as: String,

    #[builder(default)]
    cancellation: CancellationTokenHandle,

    #[builder(default)]
    ignore: Option<Gitignore>,

    #[builder(default)]
    build_result_channel: Option<UnboundedSender<BuildInfo>>,

    #[builder(default)]
    cpu_quota: Option<f64>,

    #[builder(default = "true")]
    network_enabled: bool,

    /// Build timeout, in milliseconds
    #[builder(default)]
    timeout: Option<Duration>,
}

impl BuildImageOptions {
    /// Send the specified build result to the sender, if possible
    fn send_result(&self, create_msg: impl FnOnce() -> BuildInfo) {
        if let Some(res) = self.build_result_channel.as_ref() {
            let _ = res.send(create_msg());
        }
    }
}

/// The result of building an image
pub struct BuildImageResult {}

/// Build an image from the specified [`Image`] instance.
pub async fn build_image(
    docker: Docker,
    image: &Image,
    opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    tracing::debug!(?image, "Building image");
    let timeout = opt.timeout;

    let build_job = async {
        match image {
            Image::Prebuilt { tag } => build_prebuilt_image(docker, tag, opt).await,
            Image::Dockerfile { path, file } => {
                build_image_from_dockerfile(docker, path, file.as_deref(), opt).await
            }
        }
    };

    if let Some(timeout) = timeout {
        tokio::time::timeout(timeout, build_job)
            .await
            .map_err(|_| BuildError::Timeout)
            // TODO: Replace with .flatten() when https://github.com/rust-lang/rust/issues/70142 is closed.
            .and_then(|i| i)
    } else {
        build_job.await
    }
}

async fn build_prebuilt_image(
    docker: Docker,
    tag: &str,
    opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    tracing::debug!(%tag, "Fetching prebuilt image");
    let (name, tag) = tag
        .split_once(":")
        .map_or_else(|| (tag, None), |(name, tag)| (name, Some(tag)));
    let mut create_img = docker.create_image(
        Some(CreateImageOptions {
            from_image: name,
            tag: tag.unwrap_or("latest"),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(Some(res)) = create_img
        .next()
        .with_cancel(opt.cancellation.cancelled())
        .await
    {
        let _res = res.map_err(|e| BuildError::ImagePullFailure(e.to_string()))?;
    }

    if opt.cancellation.is_cancelled() {
        return Err(BuildError::Cancelled);
    }

    Ok(BuildImageResult {})
}

async fn build_image_from_dockerfile(
    docker: Docker,
    path: &Path,
    file: Option<&str>,
    mut opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    tracing::debug!("Building image from dockerfile");
    let source_path = canonical_join(&opt.base_path, path);
    let cpu_quota = opt.cpu_quota.map(|x| (x * 100_000f64).floor() as u64);
    let cpu_period = cpu_quota.map(|_| 100_000);

    tracing::debug!(?source_path, ?file, "Building image from local folder");

    let build_options = bollard::image::BuildImageOptions {
        dockerfile: file.unwrap_or("Dockerfile"),
        t: &opt.tag_as,
        cpuquota: cpu_quota,
        cpuperiod: cpu_period,

        networkmode: if opt.network_enabled {
            "bridge"
        } else {
            "none"
        },

        rm: true,

        buildargs: [("CI", "true")].into(),

        ..Default::default()
    };

    let (tar, join_tar) = pack_as_tar(
        &source_path,
        opt.ignore.take().unwrap_or_else(Gitignore::empty),
    )
    .map_err(|e| BuildError::FileTransferError(e.to_string()))?;

    let timeout_future = match opt.timeout {
        Some(duration) => tokio::time::sleep(duration).left_future(),
        None => futures::future::pending().right_future(),
    }
    .fuse();
    tokio::pin!(timeout_future);
    let mut res = docker.build_image(build_options, None, Some(Body::wrap_stream(tar)));

    while let Some(info) = tokio::select! {
        res = res.next().with_cancel(opt.cancellation.cancelled()).map(|f| f.flatten())=> res,
        _timeout = timeout_future.as_mut() => None
    } {
        match info {
            Ok(info) => {
                if let Some(e) = info.error {
                    return Err(BuildError::BuildError {
                        error: e,
                        detail: info.error_detail,
                    });
                }
                if let Some(stream) = &info.stream {
                    tracing::debug!(stdout = %stream, "building");
                }
                opt.send_result(|| info);
            }
            Err(e) => {
                let is_recoverable = is_recoverable_error(&e);
                opt.send_result(|| {
                    let e = format!("*** Internal error when building image: {:?}", e);
                    BuildInfo {
                        error: e.into(),
                        ..Default::default()
                    }
                });

                if !is_recoverable {
                    return Err(BuildError::Internal(e.into()));
                }
            }
        }
    }

    join_tar
        .await
        .map_err(|e| {
            BuildError::Internal(
                anyhow::Error::new(e).context("Internal panic when archiving files"),
            )
        })?
        .map_err(|e| {
            BuildError::Internal(anyhow::Error::new(e).context("Failed to archive files"))
        })?;

    if timeout_future.is_terminated() {
        return Err(BuildError::Timeout);
    }
    if opt.cancellation.is_cancelled() {
        return Err(BuildError::Cancelled);
    }

    Ok(BuildImageResult {})
}
