use std::sync::{Arc, Mutex};

use async_trait::async_trait;

/// Data structure that needs to be teared down asynchronously.
///
/// This trait is used in the place of `AsyncDrop`, which is unfortunately
/// not available for now. Therefore, this trait is used to denote the need of
/// explicit teardown and maybe transfer them into another task for error handling.
///
/// Types implementing `AsyncTeardown` usually also contain a `DropBomb` which
/// prevents it from dropping without calling `teardown()`.
#[async_trait]
pub trait AsyncTeardown: Sync + Send {
    async fn teardown(&mut self);
}

/// A collector that can collect [`AsyncTeardown`] structures and
/// destruct them in reverse order.
pub struct AsyncTeardownCollector {
    items: Mutex<Vec<Arc<dyn AsyncTeardown>>>,
}

impl AsyncTeardownCollector {
    pub fn new() -> AsyncTeardownCollector {
        AsyncTeardownCollector {
            items: Mutex::new(Vec::new()),
        }
    }

    /// Add a reference-counted pointer to this collector.
    ///
    /// **The pointer MUST have only 1 reference when it's being teared down.**
    pub fn add(&self, val: Arc<dyn AsyncTeardown>) {
        let mut items = self.items.lock().expect("Failed to lock");
        items.push(val);
    }

    /// Teardown all values.
    ///
    /// # Panics
    ///
    /// **ALL collected pointers MUST have only 1 reference when they're being
    /// teared down, otherwise this function will panic.**
    pub async fn teardown_all(self) {
        let items = self.items.into_inner().expect("Failed to remove lock");
        for mut item in items.into_iter().rev() {
            Arc::get_mut(&mut item)
                .expect("Reference count of item is not 1")
                .teardown()
                .await;
        }
    }
}

impl Default for AsyncTeardownCollector {
    fn default() -> Self {
        Self::new()
    }
}
