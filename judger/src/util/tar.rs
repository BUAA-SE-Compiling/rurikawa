//! Operations related to TAR archives
//!
//!

use bytes::BytesMut;
use futures::{future::BoxFuture, FutureExt};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;
use tokio::{
    io::{self, AsyncWrite},
    task::JoinHandle,
};
use tokio_stream::Stream;
use tokio_tar::{Builder, Header};

#[tracing::instrument(skip(input))]
pub fn ignore_from_string_list<'a>(
    root: &Path,
    input: impl Iterator<Item = &'a str>,
) -> std::io::Result<Gitignore> {
    input
        .fold(GitignoreBuilder::new(&root), |mut builder, x| {
            let _ = builder
                .add_line(None, x)
                .map_err(|e| tracing::error!("Invalid ignore pattern: {}", e));
            builder
        })
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Spawn a task to pack the given `path` into a Tar file, with ignore pattern
/// supplied as `glob`.
///
/// Returns the tar file stream to read from and the join handle to the packing
/// task.
pub fn pack_as_tar(
    path: &Path,
    ignore: Gitignore,
) -> io::Result<(
    impl Stream<Item = io::Result<BytesMut>> + 'static,
    JoinHandle<io::Result<()>>,
)> {
    let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
    let read_codec = tokio_util::codec::BytesCodec::new();
    let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);

    // Own the `path` to make `tokio` happy.
    let path = path.to_owned();

    // Launch a task for archiving.
    let archiving = tokio::spawn(async move {
        let mut tar = tokio_tar::Builder::new(pipe_recv);
        add_dir_glob(&path, &path, &ignore, &mut tar).await?;
        tar.finish().await?;
        Ok(())
    });

    Ok((frame, archiving))
}

/// Add the given directory into the given tar, using the given glob pattern.
fn add_dir_glob<'a, W: AsyncWrite + Send + Sync + Unpin>(
    root: &'a Path,
    dir: &'a Path,
    glob: &'a Gitignore,
    tar: &'a mut Builder<W>,
) -> BoxFuture<'a, std::io::Result<()>> {
    async move {
        let mut read_dir = tokio::fs::read_dir(dir).await?;
        while let Some(next) = read_dir.next_entry().await? {
            let path = next.path();
            let meta = tokio::fs::metadata(&path).await?;
            if glob.matched(&path, meta.is_dir()).is_ignore() {
                continue;
            }

            if meta.is_dir() {
                add_dir_glob(root, &path, glob, tar).await?;
            } else if meta.is_file() {
                let mut file = tokio::fs::File::open(&path).await?;
                let mut header = Header::new_gnu();
                header.set_metadata(&meta);
                tar.append_data(&mut header, path.strip_prefix(root).unwrap(), &mut file)
                    .await?;
            }
        }
        Ok(())
    }
    .boxed()
}
