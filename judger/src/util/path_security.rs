//! Helper functions to disallow absolute paths or relative paths that goes into
//! parent paths.

use std::path::Path;

pub fn enforce_relative_path(path: &Path) -> Result<(), String> {
    if path.starts_with("..") {
        return Err(format!(
            "Path {} navigates into its parents, which is not allowed",
            path.to_string_lossy()
        ));
    }
    if path.is_absolute() {
        return Err(format!(
            "Path {} is an absolute path, which is not allowed",
            path.to_string_lossy()
        ));
    }

    Ok(())
}
