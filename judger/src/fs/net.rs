//! Functions to download stuff into destinations
use std::path::Path;
use tokio::{
    fs::{canonicalize, read_to_string, DirEntry, File},
    io::{AsyncWrite, AsyncWriteExt},
    process::Command,
};

#[derive(Debug)]
pub struct GitCloneOptions {
    repo: String,
    branch: Option<String>,
    depth: usize,
}

impl Default for GitCloneOptions {
    fn default() -> Self {
        GitCloneOptions {
            repo: String::new(),
            branch: Some(String::from("master")),
            depth: 5,
        }
    }
}

pub async fn git_clone(dir: Option<&Path>, options: GitCloneOptions) -> std::io::Result<()> {
    let mut clone_cmd = Command::new("git");
    clone_cmd
        .args(&["clone", &options.repo])
        .arg("--recursive")
        .arg("--single-branch")
        .arg("--shallow-submodules");
    if let Some(branch) = &options.branch {
        clone_cmd.args(&["--branch", &branch]);
    }
    if let Some(dir) = dir {
        clone_cmd.arg(dir);
    }
    let ret_result = clone_cmd.output().await?;
    if ret_result.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Git clone command failed with output:\n{}",
                String::from_utf8_lossy(&ret_result.stdout)
            ),
        ))
    }
}

pub async fn download_unzip(
    url: &str,
    dir: &Path,
    temp_file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut resp = reqwest::get(url).await?;
    let mut file = tokio::fs::File::create(temp_file_path).await?;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
    }
    drop(file);

    let unzip_res = Command::new("7z")
        .args(&[
            "x",
            &temp_file_path.to_string_lossy(),
            &format!("-o{}", dir.to_string_lossy()),
        ])
        .output()
        .await?;
    tokio::fs::remove_file(temp_file_path).await?;
    if unzip_res.status.success() {
        Ok(())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "7zip failed to extract, exited with output:\n{}",
                String::from_utf8_lossy(&unzip_res.stdout)
            ),
        )))
    }
}
