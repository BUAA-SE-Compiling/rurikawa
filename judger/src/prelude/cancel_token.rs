use std::{
    sync::atomic::AtomicBool, sync::atomic::AtomicUsize, sync::atomic::Ordering, sync::Arc,
    task::Poll, task::Waker,
};

use dashmap::DashMap;
use futures::Future;

pub struct CancellationTokenHandle {
    token_ref: Option<Arc<InnerCToken>>,
}

impl CancellationTokenHandle {
    pub fn new() -> CancellationTokenHandle {
        CancellationTokenHandle {
            token_ref: Some(Arc::new(InnerCToken {
                cancelled: AtomicBool::new(false),
                counter: AtomicUsize::new(0),
                wakers: DashMap::new(),
            })),
        }
    }

    pub fn cancel(&self) {
        if let Some(r) = self.token_ref.as_ref() {
            r.wake_all();
        }
    }

    pub fn get_token(&self) -> CancellationToken {
        CancellationToken {
            token_ref: self.token_ref.clone(),
            waker_id: None,
        }
    }

    pub fn empty() -> CancellationTokenHandle {
        Self::default()
    }
}

impl Default for CancellationTokenHandle {
    fn default() -> Self {
        CancellationTokenHandle { token_ref: None }
    }
}

struct InnerCToken {
    cancelled: AtomicBool,
    counter: AtomicUsize,
    wakers: DashMap<usize, Waker>,
}

impl InnerCToken {
    pub fn store_waker(&self, waker: Waker) -> usize {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        self.wakers.insert(id, waker);
        id
    }

    pub fn drop_waker(&self, id: usize) -> Option<Waker> {
        self.wakers.remove(&id).map(|(_id, waker)| waker)
    }

    pub fn wake_all(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.wakers
            .iter()
            .for_each(|pair| pair.value().wake_by_ref());
    }
}
pub struct CancellationToken {
    token_ref: Option<Arc<InnerCToken>>,
    waker_id: Option<usize>,
}

impl Future for CancellationToken {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(token_ref) = self.token_ref.clone() {
            if token_ref.cancelled.load(Ordering::SeqCst) {
                if let Some(id) = self.waker_id.take() {
                    token_ref.drop_waker(id);
                }
                return Poll::Ready(());
            }
            let id = token_ref.store_waker(cx.waker().clone());
            if let Some(id) = self.waker_id.take() {
                token_ref.drop_waker(id);
            }
            self.waker_id = Some(id);
            Poll::Pending
        } else {
            Poll::Pending
        }
    }
}

impl Drop for CancellationToken {
    fn drop(&mut self) {
        if let Some(token_ref) = self.token_ref.as_ref() {
            if let Some(id) = self.waker_id.take() {
                token_ref.drop_waker(id);
            }
        }
    }
}
