//! File-system-related stuff. Including manipulating test folders, performing git operations and so on.

use futures::{future::BoxFuture, Future, FutureExt, Stream, StreamExt};
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;

pub mod net;

pub const JUDGE_FILE_NAME: &str = "judge.toml";

async fn get_judge_config(root_path: &Path) -> Result<crate::config::JudgeToml, std::io::Error> {
    let judge_root_path = find_judge_root(root_path).await.unwrap();
    todo!()
}

pub fn ensure_removed_dir(path: &Path) -> BoxFuture<Result<(), std::io::Error>> {
    async move {
        let dir = read_dir(path).await?;
        dir.filter_map(|x| async move {
            if let Ok(x) = x {
                if x.file_type().await.map(|x| x.is_dir()).unwrap_or(false) {
                    Some(x.path())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .map(|x| async move { ensure_removed_dir(&x).await })
        .buffered(16usize)
        .for_each(|_| async {})
        .await;
        let res = tokio::fs::remove_dir_all(path).await;
        match &res {
            Ok(_) => {}
            Err(e) => log::error!("{:?}: {}", path, e),
        };
        res
    }
    .boxed()
}

pub fn find_judge_root(path: &Path) -> BoxFuture<Result<PathBuf, std::io::Error>> {
    async move {
        let mut dir = read_dir(path).await?;
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
                Err(e) => {
                    continue;
                }
            }
        }
        Err(std::io::ErrorKind::NotFound.into())
    }
    .boxed()
}
