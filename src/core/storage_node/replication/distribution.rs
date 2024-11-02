// ./src/core/storage_node/replication/distribution.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::console;

pub struct ReplicationDistributionManager {
    storage_node: JsValue,
    replication_threshold: u64,
    pub(crate) replication_interval: u32,
}

impl ReplicationDistributionManager {
    // Associated function to create a new instance
    pub fn new(
        storage_node: JsValue,
        replication_threshold: u64,
        replication_interval: u32,
    ) -> Self {
        Self {
            storage_node,
            replication_threshold,
            replication_interval,
        }
    }

    // Accessor for `replication_interval`
    pub fn replication_interval(&self) -> u32 {
        self.replication_interval
    }

    // Accessor for `replication_threshold`
    pub fn replication_threshold(&self) -> u64 {
        self.replication_threshold
    }
}

#[wasm_bindgen]
impl ReplicationDistributionManager {
    // JS-compatible constructor
    #[wasm_bindgen(constructor)]
    pub fn js_new(
        storage_node: JsValue,
        replication_threshold: u64,
        replication_interval: u32,
    ) -> ReplicationDistributionManager {
        ReplicationDistributionManager::new(
            storage_node,
            replication_threshold,
            replication_interval,
        )
    }

    pub async fn start_replication_distribution(&self) {
        let interval_ms = self.replication_interval();

        let f = async move {
            loop {
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            &resolve,
                            interval_ms.try_into().unwrap(),
                        )
                        .unwrap();
                });
                JsFuture::from(promise).await.unwrap();

                if let Err(e) = self.check_replication_distribution().await {
                    console::error_1(&format!("Replication distribution error: {:?}", e).into());
                }
            }
        };

        spawn_local(f);
    }

    async fn check_replication_distribution(&self) -> Result<(), JsValue> {
        let replication_threshold = self.replication_threshold();
        let replications =
            js_sys::Reflect::get(&self.storage_node, &JsValue::from_str("stored_proofs"))?;
        let replication_count = js_sys::Reflect::get(&replications, &JsValue::from_str("length"))?
            .as_f64()
            .unwrap() as u64;

        if replication_count >= replication_threshold {
            self.distribute_replications(replications).await?;
        }

        Ok(())
    }

    async fn distribute_replications(&self, replications: JsValue) -> Result<(), JsValue> {
        // Distribution logic here
        Ok(())
    }
}
