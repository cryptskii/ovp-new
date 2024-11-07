// ./src/core/storage_node/verification/challenge.rs

// Challenge Generation
// This module implements a challenge mechanism where anything suspicious can be challenged by clients or other nodes.
// Clients pay a challenge fee, reimbursed with each successful challenge, along with a portion of the slashing penalty placed on the storage node.
use crate::core::error::errors::SystemError;
use crate::core::storage_node::storage_node_contract::StorageNode;
use js_sys::Promise;
use std::sync::Arc;
use std::sync::RwLock;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{console, window};

/// A manager for handling challenge requests, evaluating suspicious activity, and issuing challenges.
pub struct ChallengeManager<RootTree, IntermediateTreeManager> {
    storage_node: Arc<RwLock<StorageNode<RootTree, IntermediateTreeManager>>>,
    challenge_fee: u64,
    challenge_threshold: u64,
    challenge_interval: u64, // interval in milliseconds
}

impl<RootTree, IntermediateTreeManager> ChallengeManager<RootTree, IntermediateTreeManager> {
    /// Initializes a new `ChallengeManager`.
    pub fn new(
        storage_node: Arc<RwLock<StorageNode<RootTree, IntermediateTreeManager>>>,
        challenge_fee: u64,
        challenge_threshold: u64,
        challenge_interval: u64,
    ) -> Self {
        Self {
            storage_node,
            challenge_fee,
            challenge_threshold,
            challenge_interval,
        }
    }

    /// Starts the challenge verification loop.
    /// Uses web_sys and js_sys for WASM-compatible async timing
    pub fn start_challenge(&self) {
        let interval = self.challenge_interval;
        let manager = self.storage_node.clone();

        let challenge_task = async move {
            loop {
                let window = window().unwrap();
                let promise = Promise::new(&mut |resolve, _| {
                    window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            &resolve,
                            interval as i32,
                        )
                        .unwrap();
                });

                JsFuture::from(promise).await.unwrap();
                if let Err(e) = Self::check_challenge(&manager).await {
                    console::error_1(&format!("Challenge error: {:?}", e).into());
                }
            }
        };

        spawn_local(challenge_task);
    }

    /// Checks if the challenge threshold has been reached and initiates challenges if necessary.
    async fn check_challenge(
        storage_node: &Arc<RwLock<StorageNode<RootTree, IntermediateTreeManager>>>,
    ) -> Result<(), SystemError> {
        let challenge_threshold = storage_node.read().unwrap().challenge_threshold;
        let challenges: Vec<ChallengeRecord> = storage_node.read().unwrap().challenges();
        let challenge_count = challenges.len();

        if challenge_count >= challenge_threshold as usize {
            // Threshold reached, initiate challenges
            Self::challenge_nodes(challenges, storage_node).await?;
        }

        Ok(())
    }

    /// Executes challenges on suspicious nodes.
    async fn challenge_nodes(
        challenges: Vec<ChallengeRecord>,
        storage_node: &Arc<RwLock<StorageNode<RootTree, IntermediateTreeManager>>>,
    ) -> Result<(), SystemError> {
        for challenge in challenges {
            // Skip nodes that are no longer in the network or already challenged
            let node_id = challenge.node_id;
            if !storage_node.read().unwrap().is_peer(&node_id) {
                continue;
            }
            if storage_node.read().unwrap().has_challenge(&node_id) {
                continue;
            }

            // Initiate challenge
            storage_node
                .write()
                .unwrap()
                .issue_challenge(node_id)
                .await?;
        }

        Ok(())
    }
}
