//! Helper functions to disallow absolute paths or relative paths that goes into
//! parent paths.

use std::path::Path;

/// Enforces a path to be a relative path that does not navigate to its parent.
pub fn enforce_relative_path(path: &Path) -> Result<(), String> {
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
