use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub last_input_seq: u32, // prevent duplicates
}

impl PlayerState {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            last_input_seq: 0,
        }
    }
}