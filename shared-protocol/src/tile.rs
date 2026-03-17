use serde::{Deserialize, Serialize};

/// A tile position on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Tile types that can exist in the mining layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileType {
    Stone,
    Wood,
    Air,
    // Add more as needed
}

/// How long (in ticks) before a mined tile respawns.
impl TileType {
    pub fn respawn_ticks(&self) -> u64 {
        match self {
            TileType::Stone => 200,     // 10 seconds at 20Hz
            TileType::Wood => 400,      // 20 seconds
            TileType::Air => 0,
        }
    }

    /// How many ticks to mine this tile.
    pub fn mine_ticks(&self) -> Option<u32> {
        match self {
            TileType::Stone => Some(20),      // 1 second
            TileType::Wood => Some(40),       // 2 seconds
            TileType::Air => None,            // Not breakable
        }
    }
}