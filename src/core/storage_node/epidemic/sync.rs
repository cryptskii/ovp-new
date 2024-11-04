// ./src/core/storage_node/epidemic/sync.rs

use crate::core::storage_node::storage_node::StorageNode;
use crate::core::types::ovp_types::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub struct SynchronizationManager<RootTree, IntermediateTreeManager> {
    storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
    synchronization_boost: AtomicU64,
    synchronization_boost_interval: Duration,
    min_synchronization_boost: u64,
    max_synchronization_boost: u64,
}
impl<RootTree, IntermediateTreeManager> SynchronizationManager<RootTree, IntermediateTreeManager> {
    pub fn new(
        storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
        synchronization_boost_interval: Duration,
        min_synchronization_boost: u64,
        max_synchronization_boost: u64,
    ) -> Self {
        Self {
            storage_node,
            synchronization_boost: AtomicU64::new(0),
            synchronization_boost_interval,
            min_synchronization_boost,
            max_synchronization_boost,
        }
    }

    pub async fn start_synchronization_boost(&self) {
        let interval_ms = self.synchronization_boost_interval.as_millis() as u32;

        let f = Closure::wrap(Box::new(move || {
            spawn_local(async move {
                if let Err(e) = self.check_synchronization_boost().await {
                    log::error!("Synchronization boost error: {:?}", e);
                }
            });
        }) as Box<dyn FnMut()>);

        let window = web_sys::window().expect("no global window exists");
        window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                f.as_ref().unchecked_ref(),
                interval_ms,
            )
            .expect("failed to set interval");
        f.forget();

        loop {
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                let window = web_sys::window().expect("no global window exists");
                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, interval_ms)
                    .expect("failed to set timeout");
            }))
            .await
            .expect("failed to await timeout");

            if let Err(e) = self.check_synchronization_boost().await {
                log::error!("Synchronization boost error: {:?}", e);
            }
        }
    }

    async fn check_synchronization_boost(&self) -> Result<(), SystemError> {
        let synchronization_boost = self.storage_node.stored_bocs.read().await.len() as u64;
        let synchronization_boost =
            synchronization_boost.saturating_sub(self.min_synchronization_boost);
        let synchronization_boost = synchronization_boost.min(self.max_synchronization_boost);

        if synchronization_boost > self.synchronization_boost.load(Ordering::Relaxed) {
            self.storage_node
                .battery_system
                .write()
                .await
                .recharge(synchronization_boost)
                .await?;
            self.synchronization_boost
                .store(synchronization_boost, Ordering::Relaxed);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SynchronizationMetrics {
    pub synchronization_boost: u64,
    pub last_check: u64,
}