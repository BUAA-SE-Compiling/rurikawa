//! Functions to download stuff into destinations

use anyhow::Context;
use async_compat::CompatExt;
use futures::TryStreamExt;
use std::{fmt::Write, path::Path};
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_tar::Archive;

#[derive(Debug)]
pub struct GitCloneOptions {
    pub repo: String,
    pub revision: String,
    // pub branch: Option<String>,
    pub depth: usize,
}

impl Default for GitCloneOptions {
    fn default() -> Self {
        GitCloneOptions {
            repo: String::new(),
            revision: String::new(),
            // branch: Some(String::from("master")),
            depth: 5,
        }
    }
}

// UNSAFE! This section calls directly into Unix `setpgrp` function to move the
// child process into a different process group, in order to avoid sending
// SIGINT into that process.
#[cfg(unix)]
extern "C" {
    fn setpgrp();
}

/// Avoid the child process from receiving SIGINT. This only works for Unix systems
/// to avoid having the child exit earlier than this process.
#[cfg(unix)]
fn set_no_sigint_handler(cmd: &mut Command) {
    unsafe {
        cmd.pre_exec(|| {
            setpgrp();
            Ok(())
        });
    }
}

/// Stub for other systems
#[cfg(not(unix))]
fn set_no_sigint_handler(_cmd: &mut Command) {}

macro_rules! do_command {
    ($($dir:expr,)? [ $cmd:expr, $($arg:expr),*]) => {
        let mut cmd = Command::new($cmd);
        cmd
            $(.current_dir($dir))?
            .args(&[$($arg),*])
            .kill_on_drop(true);
        set_no_sigint_handler(&mut cmd);

        let cmd = cmd.output().await?;

        if !cmd.status.success(){
            let mut format_string = String::new();

            write!(format_string, "Command failed: `{}",$cmd).unwrap();
            $(
                write!(format_string, " {}",$arg).unwrap();
            )*
            write!(format_string, "` returned {:?}",cmd.status.code()).unwrap();
            writeln!(format_string).unwrap();

            writeln!(format_string,"stdout: ").unwrap();
            writeln!(
                format_string,
                "{}",std::string::String::from_utf8_lossy(&cmd.stdout)).unwrap();

            writeln!(format_string,"stderr: ").unwrap();
            writeln!(
                format_string,
                "{}",std::string::String::from_utf8_lossy(&cmd.stderr)).unwrap();

            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format_string))
        }
    };
}

pub async fn git_clone(dir: &Path, options: GitCloneOptions) -> std::io::Result<()> {
    // This clone procedure follows
    // https://stackoverflow.com/questions/3489173/how-to-clone-git-repository-with-specific-revision-changeset
    // to clone a single revision. This requires the server to directly
    // specify the commit ID or it won't work.

    // # make a new blank repository in the current directory
    // git init
    //
    // # add a remote
    // git remote add origin url://to/source/repository
    //
    // # fetch a commit (or branch or tag) of interest
    // # Note: the full history up to this commit will be retrieved unless
    // #       you limit it with '--depth=...' or '--shallow-since=...'
    // git fetch origin <sha1-of-commit-of-interest>
    //
    // # reset this repository's master branch to the commit of interest
    // git reset --hard FETCH_HEAD

    tokio::fs::create_dir_all(dir).await?;

    do_command!(dir, ["git", "init"]);
    do_command!(dir, ["git", "remote", "add", "origin", &options.repo]);
    do_command!(
        dir,
        ["git", "fetch", "origin", &options.revision, "--depth", "1"]
    );
    do_command!(dir, ["git", "reset", "--hard", "FETCH_HEAD", "--"]);
    do_command!(dir, ["git", "submodule", "init"]);
    do_command!(dir, ["git", "submodule", "update", "--recommend-shallow"]);

    Ok(())
}

pub async fn download_unzip(
    client: reqwest::Client,
    req: reqwest::Request,
    dir: &Path,
) -> anyhow::Result<()> {
    log::info!("Downloading from {} to {}", req.url(), dir.display());
    let resp = client.execute(req).await?.error_for_status()?;

    let stream = resp
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        .into_async_read()
        .compat();
    let mut archive = Archive::new(stream);

    archive.unpack(dir).await?;

    Ok(())
}
