use serde::{Deserialize, Serialize};
use std::fmt::{Display, Result, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneType {
    Safe,
    PvE,
    PvP,
}

/// Identifies a specific world instance.
/// e.g. ("kremwood_1", 2) = Kremwood Forest Instance 1, Server 2
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldId {
    pub world_name: String,
    pub instance_id: u32,
}

impl WorldId {
    pub fn new(name: &str, instance: u32) -> Self {
        Self {
            world_name: name.to_string(),
            instance_id: instance,
        }
    }
}

impl Display for WorldId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}:{}", self.world_name, self.instance_id)
    }
}