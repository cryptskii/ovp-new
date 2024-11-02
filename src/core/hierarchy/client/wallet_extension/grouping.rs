// src/core/hierarchy/client/wallet_extension/grouping.rs

use crate::core::types::ovp_types::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Manages grouping of channels for the wallet extension.
pub struct GroupingManager {
    groups: Arc<RwLock<HashMap<String, ChannelGroup>>>,
}

impl GroupingManager {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new group with the given name.
    pub fn create_group(&self, group_name: &str) -> Result<(), SystemError> {
        let mut groups = self.groups.write().unwrap();
        if groups.contains_key(group_name) {
            return Err(SystemError::GroupAlreadyExists);
        }
        groups.insert(group_name.to_string(), ChannelGroup::new(group_name));
        Ok(())
    }

    /// Adds a channel to a group.
    pub fn add_channel_to_group(
        &self,
        group_name: &str,
        channel_id: &[u8; 32],
    ) -> Result<(), SystemError> {
        let mut groups = self.groups.write().unwrap();
        let group = groups
            .get_mut(group_name)
            .ok_or(SystemError::GroupNotFound)?;
        group.add_channel(channel_id)
    }

    /// Removes a channel from a group.
    pub fn remove_channel_from_group(
        &self,
        group_name: &str,
        channel_id: &[u8; 32],
    ) -> Result<(), SystemError> {
        let mut groups = self.groups.write().unwrap();
        let group = groups
            .get_mut(group_name)
            .ok_or(SystemError::GroupNotFound)?;
        group.remove_channel(channel_id)
    }

    /// Retrieves the channels in a group.
    pub fn get_group_channels(&self, group_name: &str) -> Result<Vec<[u8; 32]>, SystemError> {
        let groups = self.groups.read().unwrap();
        let group = groups.get(group_name).ok_or(SystemError::GroupNotFound)?;
        Ok(group.get_channels())
    }
}

/// Represents a group of channels.
pub struct ChannelGroup {
    name: String,
    channels: HashSet<[u8; 32]>,
}

impl ChannelGroup {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            channels: HashSet::new(),
        }
    }

    pub fn add_channel(&mut self, channel_id: &[u8; 32]) -> Result<(), SystemError> {
        if !self.channels.insert(*channel_id) {
            return Err(SystemError::ChannelAlreadyInGroup);
        }
        Ok(())
    }

    pub fn remove_channel(&mut self, channel_id: &[u8; 32]) -> Result<(), SystemError> {
        if !self.channels.remove(channel_id) {
            return Err(SystemError::ChannelNotInGroup);
        }
        Ok(())
    }

    pub fn get_channels(&self) -> Vec<[u8; 32]> {
        self.channels.iter().cloned().collect()
    }
}
