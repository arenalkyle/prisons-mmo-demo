use serde::{Deserialize, Serialize};
use crate::player::PlayerState;

/// Used for state synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub players: Vec<PlayerState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransferError {
    CombatTagged,
    WorldNotFound(String),
    WorldFull,
}