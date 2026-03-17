use serde::{Deserialize, Serialize};

/// Movement input sent from client to server each frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementInput {
    pub sequence: u32,
    pub dir_x: f32,
    pub dir_y: f32,
    pub dt: f32,
}

/// Shared movement constants (pixels per second)
pub const MOVE_SPEED: f32 = 200.0;
pub const WORLD_MIN_X: f32 = -2000.0;
pub const WORLD_MAX_X: f32 = 2000.0;
pub const WORLD_MIN_Y: f32 = -2000.0;
pub const WORLD_MAX_Y: f32 = 2000.0;
pub const MAX_DT: f32 = 0.1; // never trust the client.