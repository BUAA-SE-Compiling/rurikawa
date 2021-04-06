//! Operations related to TAR archives
//!
//!

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use async_compat::CompatExt;
use async_tar::{Builder, Header};
use bytes::{Bytes, BytesMut};
use futures::{AsyncWrite, Future, FutureExt, Stream, StreamExt};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use tokio::task::JoinHandle;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[tracing::instrument(skip(input))]
pub fn ignore_from_string_list<'a>(
    root: &Path,
    input: impl Iterator<Item = &'a str>,
) -> std::io::Result<Gitignore> {
    input
        .fold(GitignoreBuilder::new(&root), |mut builder, x| {
            match builder.add_line(None, x) {
                Ok(_) => (),
                Err(e) => tracing::error!("Invalid ignore pattern: {}", e),
            };
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
    path: PathBuf,
    ignore: Gitignore,
) -> Result<
    (
        impl Stream<Item = Result<BytesMut, std::io::Error>> + 'static,
        JoinHandle<Result<(), std::io::Error>>,
    ),
    std::io::Error,
> {
    let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
    let read_codec = tokio_util::codec::BytesCodec::new();
    let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);

    let task = async move {
        let mut tar =
            async_tar::Builder::new(futures::io::BufWriter::new(pipe_recv.compat_write()));

        add_dir_glob(&path, &path, &ignore, &mut tar).await?;
        tar.finish().await?;
        Ok(())
    };
    Ok((frame, tokio::spawn(task)))
}

/// Add the given directory into the given tar, using the given glob pattern.
fn add_dir_glob<'a, W: AsyncWrite + Send + Sync + Unpin>(
    root: &'a Path,
    dir: &'a Path,
    glob: &'a Gitignore,
    tar: &'a mut Builder<W>,
) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send + 'a>> {
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
                tar.append_data(
                    &mut header,
                    path.strip_prefix(root).unwrap(),
                    (&mut file).compat(),
                )
                .await?;
            }
        }
        Ok(())
    }
    .boxed()
}
