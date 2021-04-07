//! Helper functions to disallow absolute paths or relative paths that goes into
//! parent paths.

use std::path::Path;

/// Checks if a path is a relative path that does not navigate to its parent.
/// Returns `Err` if it's not.
pub fn enforce_child_path(path: &Path) -> Result<(), String> {
    let mut depth = 0;
    for part in path.components() {
        match part {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err(format!(
                    "Path {} is an absolute path, which is not allowed",
                    path.to_string_lossy()
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
            return Err(format!(
                "Path {} navigates into its parents, which is not allowed",
                path.to_string_lossy()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_enforce_relative_path() {
        enforce_child_path("./cat.rs".as_ref()).unwrap();
        enforce_child_path("cat.rs".as_ref()).unwrap();
        enforce_child_path("src/dog/cat.rs".as_ref()).unwrap();
        enforce_child_path("src/cat/../dog/dog.rs".as_ref()).unwrap();
        enforce_child_path("src/cat/../dog/dog.rs".as_ref()).unwrap();
        enforce_child_path("src/../dog/dog.rs".as_ref()).unwrap();
    }

    #[test]
    fn test_enforce_relative_path_fail() {
        enforce_child_path("/dog/src/dog.rs".as_ref()).unwrap_err();
        enforce_child_path("/dog.rs".as_ref()).unwrap_err();
        enforce_child_path("../dog.rs".as_ref()).unwrap_err();
        enforce_child_path("../dog/dog.rs".as_ref()).unwrap_err();
        enforce_child_path("cat/nip/../../../dog/dog.rs".as_ref()).unwrap_err();
        enforce_child_path("./cat/../../lib/dog/dog.rs".as_ref()).unwrap_err();
        enforce_child_path("./../lib/dog/dog.rs".as_ref()).unwrap_err();
    }
}
