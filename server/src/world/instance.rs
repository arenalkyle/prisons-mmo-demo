use std::collections::HashMap;
use std::sync::Arc;

use shared_protocol::movement::MovementInput;
use shared_protocol::packets::ServerPacket;
use shared_protocol::player::PlayerState;
use shared_protocol::simulation::apply_movement;
use shared_protocol::tile::TilePos;
use shared_protocol::world::WorldSnapshot;
use shared_protocol::zone::WorldId;
use tokio::sync::RwLock;
use crate::player::server_player::{MiningState, ServerPlayer};
use crate::world::spatial::SpatialGrid;
use crate::world::tile::TileManager;

const VIEW_RADIUS: f32 = 800.0;
const SPATIAL_CELL_SIZE: f32 = 256.0;

pub(crate) type SharedInstance = Arc<RwLock<WorldInstance>>;

pub(crate) struct WorldInstance {
    pub id: WorldId,
    pub tick: u64,
    pub players: HashMap<u64, ServerPlayer>,
    pub spatial: SpatialGrid,
    pub tiles: TileManager,
    max_players: usize,
}

impl WorldInstance {
    pub fn new(id: WorldId, max_players: usize) -> Self {
        Self {
            id,
            tick: 0,
            players: HashMap::new(),
            spatial: SpatialGrid::new(SPATIAL_CELL_SIZE),
            tiles: TileManager::new(),
            max_players,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_full(&self) -> bool {
        self.players.len() >= self.max_players
    }

    pub fn add_player(&mut self, player: ServerPlayer, spawn_x: f32, spawn_y: f32, ) -> bool {
        if self.is_full() {
            return false;
        }
        
        let id = player.state.id;

        // Notify nearby players about the new player
        let spawn_packet = ServerPacket::PlayerSpawned(PlayerState {
            id,
            x: spawn_x,
            y: spawn_y,
            last_input_seq: 0,
        });
        
        let spawn_bytes = Arc::new(bincode::serialize(&spawn_packet).unwrap());
        let nearby = self.spatial.query_nearby(spawn_x, spawn_y, VIEW_RADIUS);
        
        for &pid in &nearby {
            if let Some(p) = self.players.get(&pid) {
                p.send(&spawn_bytes);
            }
        }

        self.spatial.insert(id, spawn_x, spawn_y);
        self.players.insert(id, player);

        // Update spawned player's position
        if let Some(p) = self.players.get_mut(&id) {
            p.state.x = spawn_x;
            p.state.y = spawn_y;
        }

        let new_player_tx = &self.players[&id];
        
        for &pid in &nearby {
            if let Some(existing) = self.players.get(&pid) {
                let pkt = ServerPacket::PlayerSpawned(existing.state.clone());
                let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                new_player_tx.send(&bytes);
            }
        }

        let mined = self.tiles.get_mined_tiles();
        
        for pos in mined {
            let pkt = ServerPacket::TileMined { pos };
            let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
            new_player_tx.send(&bytes);
        }

        println!(
            "[{}] Player {} joined ({} in instance)",
            self.id, id, self.players.len()
        );
        true
    }

    pub fn remove_player(&mut self, id: u64) {
        if let Some(player) = self.players.remove(&id) {
            self.spatial.remove(id, player.state.x, player.state.y);

            let packet = ServerPacket::PlayerDisconnected { id };
            let bytes = Arc::new(bincode::serialize(&packet).unwrap());
            let nearby = self.spatial.query_nearby(player.state.x, player.state.y, VIEW_RADIUS);
            for &pid in &nearby {
                if let Some(p) = self.players.get(&pid) {
                    p.send(&bytes);
                }
            }

            println!(
                "[{}] Player {} left ({} in instance)",
                self.id, id, self.players.len()
            );
        }
    }

    /// Remove a player and return them (for transfers).
    pub fn take_player(&mut self, id: u64) -> Option<ServerPlayer> {
        let player = self.players.remove(&id)?;
        self.spatial.remove(id, player.state.x, player.state.y);

        let packet = ServerPacket::PlayerDisconnected { id };
        let bytes = Arc::new(bincode::serialize(&packet).unwrap());
        let nearby = self.spatial.query_nearby(player.state.x, player.state.y, VIEW_RADIUS);
        for &pid in &nearby {
            if let Some(p) = self.players.get(&pid) {
                p.send(&bytes);
            }
        }

        println!(
            "[{}] Player {} transferred out ({} in instance)",
            self.id, id, self.players.len()
        );
        Some(player)
    }

    pub fn apply_input(&mut self, player_id: u64, input: &MovementInput) {
        if let Some(player) = self.players.get_mut(&player_id) {
            let old_x = player.state.x;
            let old_y = player.state.y;

            let (nx, ny) = apply_movement(old_x, old_y, input);
            player.state.x = nx;
            player.state.y = ny;
            player.state.last_input_seq = input.sequence;

            // Cancel mining if player moved
            if player.mining.is_some() && (nx != old_x || ny != old_y) {
                player.mining = None;
                let cancel = ServerPacket::MineCancelled;
                let bytes = Arc::new(bincode::serialize(&cancel).unwrap());
                player.send(&bytes);
            }

            self.spatial.update(player_id, old_x, old_y, nx, ny);
        }
    }

    pub fn start_mining(&mut self, player_id: u64, pos: TilePos) {
        let mine_ticks = match self.tiles.mine_ticks(&pos) {
            Some(t) => t,
            None => return,
        };

        if let Some(player) = self.players.get_mut(&player_id) {
            player.mining = Some(MiningState {
                pos,
                ticks_remaining: mine_ticks,
            });
        }
    }

    pub fn stop_mining(&mut self, player_id: u64) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.mining = None;
        }
    }

    /// Process mining ticks for all active miners. Called each server tick.
    fn process_mining(&mut self) {
        let mut completed: Vec<(u64, TilePos)> = Vec::new();
        let mut cancelled: Vec<u64> = Vec::new();

        // Tick down mining progress
        for (&pid, player) in self.players.iter_mut() {
            if let Some(ref mut mining) = player.mining {
                // Check if the tile was broken by someone else
                if !self.tiles.is_tile_present(&mining.pos) {
                    cancelled.push(pid);
                    continue;
                }

                if mining.ticks_remaining <= 1 {
                    completed.push((pid, mining.pos));
                } else {
                    mining.ticks_remaining -= 1;

                    // Send progress to miner
                    let total = self.tiles.mine_ticks(&mining.pos).unwrap_or(1) as f32;
                    let progress = 1.0 - (mining.ticks_remaining as f32 / total);
                    let pkt = ServerPacket::MineProgress {
                        pos: mining.pos,
                        progress,
                    };
                    let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                    player.send(&bytes);
                }
            }
        }

        // Handle cancellations (tile broken by someone else)
        for pid in cancelled {
            if let Some(player) = self.players.get_mut(&pid) {
                player.mining = None;
                let pkt = ServerPacket::MineCancelled;
                let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                player.send(&bytes);
            }
        }

        // Handle completions
        for (pid, pos) in completed {
            if let Some(player) = self.players.get_mut(&pid) {
                player.mining = None;
            }
            
            if self.tiles.mine_tile(&pos, self.tick).is_some() {
                
                let pkt = ServerPacket::TileMined { pos };
                let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                let nearby = self.spatial.query_nearby(
                    pos.x as f32 * 32.0,
                    pos.y as f32 * 32.0,
                    VIEW_RADIUS,
                );
                
                for &npid in &nearby {
                    if let Some(p) = self.players.get(&npid) {
                        p.send(&bytes);
                    }
                }
                
            }
        }
    }

    /// Process tile respawns.
    fn process_respawns(&mut self) {
        let respawned = self.tiles.process_respawns(self.tick);
        for (pos, tile_type) in respawned {
            let pkt = ServerPacket::TileRespawned { pos, tile_type };
            let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
            let nearby = self.spatial.query_nearby(
                pos.x as f32 * 32.0,
                pos.y as f32 * 32.0,
                VIEW_RADIUS,
            );
            for &pid in &nearby {
                if let Some(p) = self.players.get(&pid) {
                    p.send(&bytes);
                }
            }
        }
    }

    /// Run one server tick. Called by the tick loop.
    pub fn tick(&mut self) {
        self.tick += 1;

        // Process systems
        self.process_mining();
        self.process_respawns();

        // Build and broadcast world snapshots (nearby only)
        self.broadcast_nearby_snapshots();
    }

    /// Spatial partitioning: only send data for entities within VIEW_RADIUS.
    fn broadcast_nearby_snapshots(&self) {
        for (&pid, player) in &self.players {
            let nearby_ids = self.spatial.query_nearby(player.state.x, player.state.y, VIEW_RADIUS);

            let nearby_states: Vec<PlayerState> = nearby_ids
                .iter()
                .filter_map(|&nid| self.players.get(&nid).map(|p| p.state.clone()))
                .collect();

            let snapshot = WorldSnapshot {
                tick: self.tick,
                players: nearby_states,
            };
            let packet = ServerPacket::WorldSnapshot(snapshot);
            let bytes = Arc::new(bincode::serialize(&packet).unwrap());
            player.send(&bytes);
        }
    }
}