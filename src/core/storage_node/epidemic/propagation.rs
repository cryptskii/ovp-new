use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::core::storage_node::storage_node_contract::StorageNode;

pub struct BatteryPropagation {
    storage_node: Arc<StorageNode>,
    battery_level: AtomicU64,
    battery_boost: AtomicU64,
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
    synchronization_boost: AtomicU64,
    synchronization_boost_interval: Duration,
}

impl BatteryPropagation {
    pub fn new(
        storage_node: Arc<StorageNode>,
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
        }
    }

    pub async fn start_battery_propagation(
        self: Arc<Self>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let interval_ms = self.charge_interval.as_millis() as u32;

        let f = Closure::wrap(Box::new({
            let self_clone = Arc::clone(&self);
            move || {
                let self_clone = Arc::clone(&self_clone);
                spawn_local(async move {
                    if let Err(e) = self_clone.propagate_charging().await {
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

            if let Err(e) = self.propagate_charging().await {
                log::error!("Battery propagation error: {:?}", e);
                return Err(e.into());
            }
        }
    }

    async fn propagate_charging(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Implement actual charging propagation logic here
        Ok(())
    }

    pub fn get_battery_level(&self) -> u64 {
        self.battery_level.load(Ordering::Relaxed)
    }

    pub fn get_battery_boost(&self) -> u64 {
        self.battery_boost.load(Ordering::Relaxed)
    }

    pub fn get_synchronization_boost(&self) -> u64 {
        self.synchronization_boost.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_propagation_initialization() {
        // TODO: Add tests
    }
}
