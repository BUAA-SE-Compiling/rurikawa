//! Helper functions to disallow absolute paths or relative paths that goes into
//! parent paths.

use futures::prelude::*;
use std::path::Path;
use tracing::warn;

/// Checks if a path is a relative path that does not navigate to its parent.
/// Returns `Err` if it's not.
pub fn assert_child_path(path: &Path) -> std::io::Result<()> {
    let mut depth = 0;
    for part in path.components() {
        match part {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "Path {} is an absolute path, which is not allowed",
                        path.to_string_lossy()
                    ),
                ));
            }
            std::path::Component::CurDir => {
                // no-op.
            }
            std::path::Component::ParentDir => {
                depth -= 1;
            }
            std::path::Component::Normal(_) => {
                depth += 1;
            }
        }
        if depth < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Path {} navigates into parent, which is not allowed",
                    path.to_string_lossy()
                ),
            ));
        }
    }
    Ok(())
}

/// Checks if any parent of the given path is a symbolic link, and returns `Err`
/// if that's true.
pub async fn assert_no_symlink_in_path(path: &Path) -> std::io::Result<()> {
    // TODO: Add `.buffered(...)` when this compiler issue gets repaired:
    // https://github.com/rust-lang/rust/issues/64552
    futures::stream::iter(path.ancestors().map(Ok))
        .try_for_each(assert_not_symlink)
        .await
}

async fn assert_not_symlink(path: &Path) -> std::io::Result<()> {
    let metadata = tokio::fs::metadata(path).await;
    let metadata = match metadata {
        Err(e) => {
            warn!(
                "Non-existent path when asserting not symlink: {}; err: {}",
                path.to_string_lossy(),
                e
            );
            return Ok(());
        }
        Ok(m) => m,
    };
    if metadata.file_type().is_symlink() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path {} is a symbolic link.", path.to_string_lossy()),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_enforce_relative_path() {
        assert_child_path("./cat.rs".as_ref()).unwrap();
        assert_child_path("cat.rs".as_ref()).unwrap();
        assert_child_path("src/dog/cat.rs".as_ref()).unwrap();
        assert_child_path("src/cat/../dog/dog.rs".as_ref()).unwrap();
        assert_child_path("src/cat/../dog/dog.rs".as_ref()).unwrap();
        assert_child_path("src/../dog/dog.rs".as_ref()).unwrap();
    }

    #[test]
    fn test_enforce_relative_path_fail() {
        assert_child_path("/dog/src/dog.rs".as_ref()).unwrap_err();
        assert_child_path("/dog.rs".as_ref()).unwrap_err();
        assert_child_path("../dog.rs".as_ref()).unwrap_err();
        assert_child_path("../dog/dog.rs".as_ref()).unwrap_err();
        assert_child_path("cat/nip/../../../dog/dog.rs".as_ref()).unwrap_err();
        assert_child_path("./cat/../../lib/dog/dog.rs".as_ref()).unwrap_err();
        assert_child_path("./../lib/dog/dog.rs".as_ref()).unwrap_err();
    }
}
