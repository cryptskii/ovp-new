use crate::core::{
    error::errors::SystemErrorType, storage_node::storage_node_contract::StorageNodeConfig,
};
use std::sync::atomic::{AtomicU64, Ordering};
use web_sys::window;

pub struct BatteryChargingSystem {
    pub battery_level: AtomicU64,
    config: StorageNodeConfig,
    last_charge_time: AtomicU64,
    pub reward_multiplier: AtomicU64,
    pub stake_amount: AtomicU64,
}

impl BatteryChargingSystem {
    pub fn new(config: StorageNodeConfig, stake_amount: u64) -> Self {
        let now = window().unwrap().performance().unwrap().now() as u64;
        Self {
            battery_level: AtomicU64::new(config.battery_config.max_battery),
            config,
            last_charge_time: AtomicU64::new(now),
            reward_multiplier: AtomicU64::new(100),
            stake_amount: AtomicU64::new(stake_amount),
        }
    }

    pub async fn charge_for_processing(&self) -> Result<(), SystemErrorType> {
        let current_level = self.battery_level.load(Ordering::Acquire);
        if current_level < self.config.battery_config.minimum_battery {
            return Err(SystemErrorType::InsufficientBalance);
        }
        let now = window().unwrap().performance().unwrap().now() as u64;
        let last_charge = self.last_charge_time.load(Ordering::Acquire);
        if now - last_charge < self.config.battery_config.charge_cooldown {
            return Err(SystemErrorType::SpendingLimitExceeded);
        }
        self.battery_level
            .fetch_sub(self.config.battery_config.discharge_rate, Ordering::Release);
        self.update_reward_multiplier();
        Ok(())
    }

    pub async fn recharge(&self, synchronized_nodes: u64) -> Result<(), SystemErrorType> {
        let now = window().unwrap().performance().unwrap().now() as u64;

        let last_charge = self.last_charge_time.load(Ordering::Acquire);
        if now - last_charge < self.config.battery_config.charge_cooldown {
            return Err(SystemErrorType::SpendingLimitExceeded);
        }

        let current_level = self.battery_level.load(Ordering::Acquire);

        if current_level >= self.config.battery_config.max_battery {
            return Ok(());
        }

        let charge_rate = self.config.battery_config.charge_rate * synchronized_nodes;
        let new_level = current_level.saturating_add(charge_rate);
        let capped_level = new_level.min(self.config.battery_config.max_battery);

        self.battery_level.store(capped_level, Ordering::Release);
        self.last_charge_time.store(now, Ordering::Release);
        self.update_reward_multiplier();
        Ok(())
    }

    fn update_reward_multiplier(&self) {
        let battery_percentage = self.get_charge_percentage();
        let multiplier = if battery_percentage >= 98.0 {
            100
        } else if battery_percentage >= 80.0 {
            battery_percentage as u64
        } else {
            0
        };
        self.reward_multiplier.store(multiplier, Ordering::Release);
    }

    pub fn get_charge_percentage(&self) -> f64 {
        let current_level = self.battery_level.load(Ordering::Relaxed);
        (current_level as f64 / self.config.battery_config.max_battery as f64) * 100.0
    }

    pub async fn wait_for_sufficient_charge(&self) -> Result<(), SystemErrorType> {
        let mut attempts = 0;

        while self.battery_level.load(Ordering::Acquire) < self.config.battery_config.min_battery {
            if attempts >= self.config.battery_config.max_charge_attempts {
                return Err(SystemErrorType::SpendingLimitExceeded);
            }
            if self.battery_level.load(Ordering::Acquire) == 0 {
                return Err(SystemErrorType::InsufficientBalance);
            }
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        &resolve,
                        self.config.battery_config.charge_wait_ms as i32,
                    )
                    .unwrap();
            });
            wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
            attempts += 1;
        }
        Ok(())
    }

    pub fn get_reward_multiplier(&self) -> u64 {
        self.reward_multiplier.load(Ordering::Relaxed)
    }

    pub fn get_stake_amount(&self) -> u64 {
        self.stake_amount.load(Ordering::Relaxed)
    }
}
