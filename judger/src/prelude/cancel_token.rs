use async_trait::async_trait;
use futures::{pin_mut, Future, FutureExt};

// as tokio_util releases v0.4.0, these types are no longer required.

pub type CancellationTokenHandle = tokio_util::sync::CancellationToken;

pub type CancellationToken<'a> = tokio_util::sync::WaitForCancellationFuture<'a>;

#[async_trait]
pub trait CancelFutureExt {
    type Output;

    /// Execute this task with the given cancellation token, returning `None`
    /// if the task is being cancelled and `Some(output)` otherwise.
    async fn with_cancel(self, mut cancel: CancellationToken<'_>) -> Option<Self::Output>;
}

#[async_trait]
impl<T> CancelFutureExt for T
where
    T: Future + Send,
{
    type Output = T::Output;

    async fn with_cancel(self, cancel: CancellationToken<'_>) -> Option<T::Output> {
        let self_ = self.fuse();
        pin_mut!(self_);

        tokio::select! {
            _abort = cancel => None,
            fut = self_ => Some(fut)
        }
    }
}
