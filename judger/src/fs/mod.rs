//! File-system-related stuff. Including manipulating test folders, performing git operations and so on.

use futures::{future::BoxFuture, Future, FutureExt};
use std::path::{Path, PathBuf};
use tokio::fs::read_dir;
use tokio::stream::{Stream, StreamExt};

pub mod net;

const JUDGE_FILE_NAME: &str = "judge.toml";

async fn get_judge_config(root_path: &Path) -> Result<crate::config::JudgeToml, std::io::Error> {
    let judge_root_path = find_judge_root(root_path).await.unwrap();
    todo!()
}

pub fn find_judge_root(path: &Path) -> BoxFuture<Result<PathBuf, std::io::Error>> {
    async move {
        let dir = read_dir(path).await?;
        let mut contents = dir.collect::<Result<Vec<_>, _>>().await?;
        let mut dirs = vec![];
        let mut files = vec![];
        for content in contents.drain(..) {
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
                    if e.kind() == std::io::ErrorKind::NotFound {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Err(std::io::ErrorKind::NotFound.into())
    }
    .boxed()
}
