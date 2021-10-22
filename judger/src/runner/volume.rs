//! Code for sharing files between different containers

use std::path::Path;

use crate::util::tar::pack_as_tar;
use async_trait::async_trait;
use bollard::{
    container::{RemoveContainerOptions, UploadToContainerOptions},
    image::CreateImageOptions,
    models::{Mount, MountTypeEnum},
    volume::{CreateVolumeOptions, RemoveVolumeOptions},
    Docker,
};
use drop_bomb::DropBomb;
use futures::TryStreamExt;
use ignore::gitignore::Gitignore;

pub struct Volume {
    docker: Docker,
    volume: bollard::models::Volume,

    _drop_bomb: DropBomb,
}

impl Volume {
    pub async fn create(docker: Docker, name: String) -> Result<Self, bollard::errors::Error> {
        tracing::debug!(%name, "Creating volume");
        let vol_res = docker
            .create_volume(CreateVolumeOptions {
                name: name.as_str(),
                driver: "local",
                ..Default::default()
            })
            .await?;

        Ok(Self {
            docker,
            volume: vol_res,

            _drop_bomb: DropBomb::new("`Volume::teardown()` must be called before dropping!"),
        })
    }

    pub async fn copy_local_files_into(
        &self,
        path: &Path,
        ignore: Gitignore,
    ) -> anyhow::Result<()> {
        tracing::debug!(?path, %self.volume.name, "Copying local files into volume");

        let (stream, join) = pack_as_tar(path, ignore)?;

        // Ensure the dummy image is present.
        // We use busybox as the dummy image here.
        self.docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: "busybox",
                    tag: "latest",
                    ..Default::default()
                }),
                None,
                None,
            )
            .map_ok(|_| ())
            .try_collect::<()>()
            .await?;

        let mount = self.as_mount("/files/", false);
        let container = self
            .docker
            .create_container::<String, String>(
                None,
                bollard::container::Config {
                    image: Some("busybox".into()),
                    host_config: Some(bollard::service::HostConfig {
                        mounts: Some(vec![mount]),
                        ..Default::default()
                    }),

                    ..Default::default()
                },
            )
            .await?;

        let res = self
            .docker
            .upload_to_container(
                &container.id,
                Some(UploadToContainerOptions {
                    path: "/files/",
                    no_overwrite_dir_non_dir: "false",
                }),
                hyper::Body::wrap_stream(stream),
            )
            .await;

        self.docker
            .remove_container(
                &container.id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;

        join.await??;
        res.map_err(|e| e.into())
    }

    pub async fn remove(&mut self) -> Result<(), bollard::errors::Error> {
        tracing::debug!(%self.volume.name, "Removing volume");

        // Defuse the teardown drop bomb.
        // It's not our fault if Docker blows up at this point (*/ω＼*)
        self._drop_bomb.defuse();

        self.docker
            .remove_volume(&self.volume.name, Some(RemoveVolumeOptions { force: true }))
            .await?;

        Ok(())
    }

    pub fn as_mount(&self, on_path: &str, readonly: bool) -> Mount {
        Mount {
            target: Some(on_path.to_string()),
            source: Some(self.volume.name.to_string()),
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(readonly),
            ..Default::default()
        }
    }
}

#[async_trait]
impl crate::util::AsyncTeardown for Volume {
    async fn teardown(&mut self) {
        let _ = self.remove().await;
    }
}
