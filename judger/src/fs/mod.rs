//! File-system-related stuff. Including manipulating test folders, performing git operations and so on.

use futures::{future::BoxFuture, prelude::*};
use respector::prelude::*;
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;

pub mod net;

pub const JUDGE_FILE_NAME: &str = "judge.toml";

/// Remove a directory recursively.
pub fn ensure_removed_dir(path: &Path) -> BoxFuture<std::io::Result<()>> {
    async move {
        let entries = match read_dir(path).await {
            Ok(dir) => tokio_stream::wrappers::ReadDirStream::new(dir),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => return Ok(()),
                _ => return Err(e),
            },
        };
        entries
            .filter_map(|entry| async move {
                let entry = entry.ok()?;
                let metadata = entry.metadata().await.ok()?;
                let mut permissions = metadata.permissions();
                permissions.set_readonly(false);
                let _ = tokio::fs::set_permissions(entry.path(), permissions).await;
                metadata.file_type().is_dir().then(|| entry.path())
            })
            .map(|dir| async move { ensure_removed_dir(&dir).await })
            .buffered(16usize)
            .for_each(|_| async {})
            .await;
        tokio::fs::remove_dir_all(path)
            .await
            .inspect_err(|e| log::error!("{:?}: {}", path, e))
    }
    .boxed()
}

pub fn find_judge_root(path: &Path) -> BoxFuture<std::io::Result<PathBuf>> {
    async move {
        let mut dir = tokio_stream::wrappers::ReadDirStream::new(read_dir(path).await?);
        let mut dirs = vec![];
        let mut files = vec![];
        while let Some(content) = dir.next().await {
            let content = content?;
            if content.file_type().await?.is_dir() {
                dirs.push(content);
            } else {
                files.push(content)
            }
        }
        for f in files {
            if f.file_name() == JUDGE_FILE_NAME {
                return Ok(path.into());
            }
        }
        for d in dirs {
            match find_judge_root(&d.path()).await {
                Ok(res) => return Ok(res),
                Err(_e) => {
                    continue;
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cannot find any folder that contains `judge.toml`.",
        ))
    }
    .boxed()
}
