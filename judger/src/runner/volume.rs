//! Code for sharing files between different containers

use std::{collections::HashMap, path::Path};

use bollard::{
    container::{RemoveContainerOptions, UploadToContainerOptions},
    models::Mount,
    volume::{CreateVolumeOptions, RemoveVolumeOptions},
    Docker,
};
use drop_bomb::DropBomb;
use ignore::gitignore::Gitignore;
use scopeguard::defer;

use crate::{runner::exec::CreateContainerConfig, util::tar::pack_as_tar};

pub struct Volume {
    docker: Docker,
    volume: bollard::models::Volume,

    _drop_bomb: DropBomb,
}

impl Volume {
    pub async fn create(docker: Docker, name: String) -> Result<Self, bollard::errors::Error> {
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
        let (stream, join) = pack_as_tar(path, ignore)?;

        let container = self
            .docker
            .create_container::<String, _>(
                None,
                bollard::container::Config {
                    volumes: Some(
                        Some((self.volume.name.clone(), HashMap::new()))
                            .into_iter()
                            .collect(),
                    ),
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

    pub async fn teardown(&mut self) -> Result<(), bollard::errors::Error> {
        // Defuse the teardown drop bomb.
        // It's not our fault if Docker blows up at this point (*/ω＼*)
        self._drop_bomb.defuse();

        self.docker
            .remove_volume(&self.volume.name, Some(RemoveVolumeOptions { force: true }))
            .await?;

        Ok(())
    }
}
