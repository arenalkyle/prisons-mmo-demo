use std::collections::HashMap;
use shared_protocol::tile::{TilePos, TileType};

/// Manages the mining tile layer for a world instance.
pub(crate) struct TileManager {
    /// Original tile definitions
    original_tiles: HashMap<TilePos, TileType>,
    /// Currently mined tiles and when they respawn (server tick).
    mined_tiles: HashMap<TilePos, u64>,
}

impl TileManager {
    pub fn new() -> Self {
        Self {
            original_tiles: HashMap::new(),
            mined_tiles: HashMap::new(),
        }
    }

    /// Register a mineable tile
    pub fn add_tile(&mut self, pos: TilePos, tile_type: TileType) {
        self.original_tiles.insert(pos, tile_type);
    }

    /// Check if a tile exists and is not currently mined
    pub fn is_tile_present(&self, pos: &TilePos) -> bool {
        self.original_tiles.contains_key(pos) && !self.mined_tiles.contains_key(pos)
    }

    /// Get the tile type at a position
    pub fn get_tile(&self, pos: &TilePos) -> Option<TileType> {
        if self.mined_tiles.contains_key(pos) {
            None
        } else {
            self.original_tiles.get(pos).copied()
        }
    }

    /// Get the mine duration in ticks for a tile.
    pub fn mine_ticks(&self, pos: &TilePos) -> Option<u32> {
        self.get_tile(pos).map(|t| t.mine_ticks())?
    }

    /// Mark a tile as mined. Returns the respawn tick if successful.
    pub fn mine_tile(&mut self, pos: &TilePos, current_tick: u64) -> Option<u64> {
        if let Some(tile_type) = self.get_tile(pos) {
            let respawn_tick = current_tick + tile_type.respawn_ticks();
            self.mined_tiles.insert(*pos, respawn_tick);
            Some(respawn_tick)
        } else {
            None
        }
    }

    /// Process respawns.
    pub fn process_respawns(&mut self, current_tick: u64) -> Vec<(TilePos, TileType)> {
        let mut respawned = Vec::new();
        self.mined_tiles.retain(|pos, respawn_tick| {
            if current_tick >= *respawn_tick {
                if let Some(&tile_type) = self.original_tiles.get(pos) {
                    respawned.push((*pos, tile_type));
                }
                false
            } else {
                true
            }
        });
        respawned
    }

    /// Get all currently mined tile positions (for players entering the area).
    pub fn get_mined_tiles(&self) -> Vec<TilePos> {
        self.mined_tiles.keys().copied().collect()
    }
}