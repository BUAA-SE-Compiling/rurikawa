use std::path::{Path, PathBuf};

use crate::config::Image;
use crate::util::tar::pack_as_tar;

use bollard::{image::CreateImageOptions, models::BuildInfo, Docker};
use derive_builder::Builder;
use hyper::Body;
use ignore::gitignore::Gitignore;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;

use super::super::{model::canonical_join, BuildError};

#[derive(Builder, Debug)]
pub struct BuildImageOptions {
    docker: Docker,

    base_path: PathBuf,

    tag_as: String,

    #[builder(default)]
    ignore: Option<Gitignore>,

    #[builder(default)]
    build_result_channel: Option<UnboundedSender<BuildInfo>>,
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
    image: &Image,
    opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    match image {
        Image::Prebuilt { tag } => build_prebuilt_image(tag, opt).await,
        Image::Dockerfile { path, file } => {
            build_image_from_dockerfile(path, file.as_deref(), opt).await
        }
    }
}

async fn build_prebuilt_image(
    tag: &str,
    opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    let mut create_img = opt.docker.create_image(
        Some(CreateImageOptions {
            from_image: tag,
            tag: &opt.tag_as,
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(res) = create_img.next().await {
        let _res = res.map_err(|e| BuildError::ImagePullFailure(e.to_string()))?;
    }

    Ok(BuildImageResult {})
}

async fn build_image_from_dockerfile(
    path: &Path,
    file: Option<&str>,
    mut opt: BuildImageOptions,
) -> Result<BuildImageResult, BuildError> {
    let source_path = canonical_join(&opt.base_path, path);
    let build_options = bollard::image::BuildImageOptions {
        dockerfile: file.unwrap_or("Dockerfile"),
        t: &opt.tag_as,
        ..Default::default()
    };

    let (tar, join_tar) = pack_as_tar(
        &source_path,
        opt.ignore.take().unwrap_or_else(Gitignore::empty),
    )
    .map_err(|e| BuildError::FileTransferError(e.to_string()))?;

    let mut res = opt
        .docker
        .build_image(build_options, None, Some(Body::wrap_stream(tar)));

    while let Some(info) = res.next().await {
        match info {
            Ok(info) => {
                if let Some(e) = info.error {
                    return Err(BuildError::BuildError {
                        error: e,
                        detail: info.error_detail,
                    });
                }
                opt.send_result(|| info);
            }
            Err(e) => {
                let is_recoverable = matches!(
                    &e,
                    bollard::errors::Error::JsonDataError { .. }
                        | bollard::errors::Error::JsonSerdeError { .. }
                        | bollard::errors::Error::StrParseError { .. }
                        | bollard::errors::Error::StrFmtError { .. }
                        | bollard::errors::Error::URLEncodedError { .. }
                );
                opt.send_result(|| {
                    let e = format!("*** Internal error when building image: {:?}", e);
                    BuildInfo {
                        error: e.into(),
                        ..Default::default()
                    }
                });

                if !is_recoverable {
                    return Err(BuildError::Internal(format!("{:?}", e)));
                }
            }
        }
    }

    join_tar
        .await
        .map_err(|e| BuildError::Internal(format!("Internal panic when archiving files: {}", e)))?
        .map_err(|e| BuildError::Internal(format!("Failed to archive files: {}", e)))?;

    Ok(BuildImageResult {})
}
