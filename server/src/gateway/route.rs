use std::collections::HashMap;
use std::sync::Arc;
use shared_protocol::zone::WorldId;
use tokio::sync::RwLock;

use crate::world::instance::{SharedInstance, WorldInstance};

/// The gateway manages all world instances and routes players to them.
pub(crate) struct Gateway {
    instances: HashMap<WorldId, SharedInstance>,
}

pub(crate) type SharedGateway = Arc<RwLock<Gateway>>;

impl Gateway {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Register a world instance.
    pub fn register_instance(&mut self, instance: WorldInstance) {
        let id = instance.id.clone();
        let shared = Arc::new(RwLock::new(instance));
        self.instances.insert(id, shared);
    }

    /// Get a world instance by ID.
    pub fn get_instance(&self, id: &WorldId) -> Option<SharedInstance> {
        self.instances.get(id).cloned()
    }

    /// Find the least populated instance for a given world name.
    pub fn find_best_instance(&self, world_name: &str) -> Option<SharedInstance> {
        self.instances
            .iter()
            .filter(|(wid, _)| wid.world_name == world_name)
            .min_by_key(|(_, inst)| {
                inst.try_read()
                    .map(|i| i.player_count())
                    .unwrap_or(usize::MAX)
            })
            .map(|(_, inst)| inst.clone())
    }

    /// List all instance IDs.
    pub fn list_instances(&self) -> Vec<WorldId> {
        self.instances.keys().cloned().collect()
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new()
    }
}