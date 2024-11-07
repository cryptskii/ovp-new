use crate::core::error::errors::SystemError;
use crate::core::storage_node::storage_node_contract::StorageNode;
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
pub struct ResponseVerificationManagerWrapper(ResponseVerificationManager);

#[wasm_bindgen]
impl ResponseVerificationManagerWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(
        storage_node: JsValue,
        response_threshold: u64,
        response_interval: u64,
    ) -> Result<ResponseVerificationManagerWrapper, JsValue> {
        let storage_node: Arc<StorageNode<(), ()>> = serde_wasm_bindgen::from_value(storage_node)?;
        Ok(ResponseVerificationManagerWrapper(
            ResponseVerificationManager {
                storage_node,
                response_threshold,
                response_interval,
            },
        ))
    }

    pub fn start_response_verification(&self) {
        self.0.start_response_verification();
    }
}

impl ResponseVerificationManager {
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
                if elapsed < manager.response_interval as f64 {
                    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(
                        &mut |resolve, _| {
                            window
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    &resolve,
                                    (manager.response_interval as f64 - elapsed) as i32,
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
        let responses = self.storage_node.retrieve_boc(&[0u8; 32]).await?;
        let response_count = responses.len();

        if response_count >= response_threshold {
            self.verify_responses(responses).await?;
        }

        Ok(())
    }

    async fn verify_responses(&self, responses: Vec<[u8; 32]>) -> Result<(), SystemError> {
        for proof in responses {
            if !self.storage_node.retrieve_proof(&proof).await? {
                // Assuming there's no apply_penalty method, we'll just log the error
                console::error_1(&format!("Invalid proof: {:?}", proof).into());
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
