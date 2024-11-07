use crate::core::error::errors::SystemError;
use crate::core::storage_node::storage_node::StorageNode;
use crate::core::types::ZkProof;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

pub trait Duration {
    fn as_millis(&self) -> u64;
}
pub struct ResponseVerificationManager {
    storage_node: Arc<StorageNode<(), ()>>,
    response_threshold: u64,
    response_interval: u64,
}

#[wasm_bindgen]
impl ResponseVerificationManager {
    #[wasm_bindgen(constructor)]
    pub fn new(
        storage_node: Arc<StorageNode<(), ()>>,
        response_threshold: u64,
        response_interval: u64,
    ) -> Self {
        Self {
            storage_node,
            response_threshold,
            response_interval,
        }
    }

    pub fn start_response_verification(&self) {
        let manager = self.clone();
        spawn_local(async move {
            let window = web_sys::window().expect("no global window exists");
            let performance = window
                .performance()
                .expect("performance should be available");

            loop {
                let start_time = performance.now();
                if let Err(e) = manager.check_response_verification().await {
                    console::error_1(&format!("Response verification error: {:?}", e).into());
                }

                // Simulate interval using performance.now()
                let elapsed = performance.now() - start_time;
                if elapsed < manager.response_interval.as_millis() as f64 {
                    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(
                        &mut |resolve, _| {
                            window
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    &resolve,
                                    (manager.response_interval.as_millis() as f64 - elapsed) as i32,
                                )
                                .expect("should register timeout");
                        },
                    ))
                    .await
                    .expect("timeout should complete");
                }
            }
        });
    }

    async fn check_response_verification(&self) -> Result<(), SystemError> {
        let response_threshold = self.response_threshold;
        let responses = self.storage_node.as_ref().get_responses().await?;
        let response_count = responses.len();

        if response_count >= response_threshold {
            self.verify_responses(responses).await?;
        }

        Ok(())
    }

    async fn verify_responses(&self, responses: Vec<ZkProof>) -> Result<(), SystemError> {
        for proof in responses {
            if !self.storage_node.as_ref().verify_proof(&proof)? {
                self.storage_node
                    .as_ref()
                    .penalize_node(proof.node_id)
                    .await?;
                break;
            }
        }

        Ok(())
    }
}

impl Clone for ResponseVerificationManager {
    fn clone(&self) -> Self {
        Self {
            storage_node: Arc::clone(&self.storage_node),
            response_threshold: self.response_threshold,
            response_interval: self.response_interval,
        }
    }
}
