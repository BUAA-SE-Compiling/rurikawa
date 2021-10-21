use bytes::{Bytes, BytesMut};
use futures::{Future, FutureExt};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use tokio::io::AsyncWrite;
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use tokio_tar::{Builder, Header};

pub fn tar_with_files<'a>(
    files: impl Iterator<Item = (String, Bytes)> + Send + 'static,
) -> (
    impl Stream<Item = Result<BytesMut, std::io::Error>> + 'static,
    JoinHandle<()>,
) {
    let (pipe_recv, pipe_send) = tokio::io::duplex(8192);
    let read_codec = tokio_util::codec::BytesCodec::new();
    let frame = tokio_util::codec::FramedRead::new(pipe_send, read_codec);

    let archiving = tokio::spawn(async move {
        let mut tar = tokio_tar::Builder::new(pipe_recv);
        for (name, file) in files {
            let mut header = Header::new_gnu();
            header.set_path(name).unwrap();
            header.set_mode(0o777);
            tar.append(&header, &*file)
                .await
                .expect("Failed to append file");
        }
        tar.finish().await.expect("Failed to finish tar");
    });

    (frame, archiving)
}
