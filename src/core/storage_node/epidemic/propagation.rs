// ./src/core/storage_node/epidemic/propagation.rs

// Battery Charging Propagation
// This module implements the battery charging protocol for maintaining node synchronization.
// It uses a battery-based mechanism to ensure nodes remain synchronized and properly
// overlapping with other nodes in the network.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::core::storage_node::storage_node_contract::StorageNode;

pub struct BatteryPropagation<RootTree, Contract> {
    storage_node: Arc<StorageNode<RootTree, Contract>>, // Storage node
    battery_level: AtomicU64,                           // 0-100%
    battery_boost: AtomicU64,                           // 0-100%
    charge_interval: Duration,                          // Interval for charging
    optimal_threshold: f64,                             // 98%
    high_threshold: f64,                                // 80%
    suspension_threshold: f64,                          // 0%
    charge_rate: f64,                 // Rate of charging based on overlapping nodes
    discharge_rate: f64,              // Rate of discharge when out of sync
    suspension_duration: Duration,    // Duration for suspension
    min_nodes_for_charging: u64,      // Minimum number of nodes for charging
    min_battery: u64,                 // Minimum battery level for charging
    max_battery: u64,                 // Maximum battery level for charging
    max_charge_attempts: u64,         // Maximum number of charge attempts
    max_synchronization_boost: u64,   // Maximum synchronization boost
    min_synchronization_boost: u64,   // Minimum synchronization boost
    synchronization_boost: AtomicU64, // Synchronization boost
    synchronization_boost_interval: Duration, // Synchronization boost interval
    _phantom: std::marker::PhantomData<Contract>, // Phantom data to use Contract
}
impl<RootTree, Contract> BatteryPropagation<RootTree, Contract> {
    pub fn new(
        storage_node: Arc<StorageNode<RootTree, Contract>>,
        charge_interval: Duration,
        optimal_threshold: f64,
        high_threshold: f64,
        suspension_threshold: f64,
        charge_rate: f64,
        discharge_rate: f64,
        suspension_duration: Duration,
        min_nodes_for_charging: u64,
        min_battery: u64,
        max_battery: u64,
        max_charge_attempts: u64,
        max_synchronization_boost: u64,
        min_synchronization_boost: u64,
        synchronization_boost_interval: Duration,
    ) -> Self {
        Self {
            storage_node,
            battery_level: AtomicU64::new(0),
            battery_boost: AtomicU64::new(0),
            charge_interval,
            optimal_threshold,
            high_threshold,
            suspension_threshold,
            charge_rate,
            discharge_rate,
            suspension_duration,
            min_nodes_for_charging,
            min_battery,
            max_battery,
            max_charge_attempts,
            max_synchronization_boost,
            min_synchronization_boost,
            synchronization_boost: AtomicU64::new(0),
            synchronization_boost_interval,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn start_battery_propagation(
        self: Arc<Self>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        RootTree: 'static,
        Contract: 'static,
    {
        let interval_ms = self.charge_interval.as_millis() as u32;

        let f = Closure::wrap(Box::new({
            let self_clone = Arc::clone(&self);
            move || {
                let self_clone = Arc::clone(&self_clone);
                spawn_local(async move {
                    if let Err(e) = self_clone.start_battery_propagation().await {
                        log::error!("Battery propagation error: {:?}", e);
                    }
                });
            }
        }) as Box<dyn FnMut()>);

        let window = web_sys::window().expect("no global window exists");
        window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                f.as_ref().unchecked_ref(),
                interval_ms.try_into().unwrap(),
            )
            .expect("failed to set interval");
        f.forget();

        loop {
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                let window = web_sys::window().expect("no global window exists");
                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        &resolve,
                        interval_ms.try_into().unwrap(),
                    )
                    .expect("failed to set timeout");
            }))
            .await
            .expect("failed to await timeout");

            if let Err(e) = Arc::clone(&self).start_battery_propagation().await {
                log::error!("Battery propagation error: {:?}", e);
                return Err(e.into());
            }
        }
    }
}
