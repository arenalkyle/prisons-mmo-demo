use serde::{Deserialize, Serialize};
use crate::movement::MovementInput;
use crate::player::PlayerState;
use crate::tile::{TilePos, TileType};
use crate::world::{TransferError, WorldSnapshot};
use crate::zone::WorldId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    /// Player movement input
    MoveInput(MovementInput),
    /// Start mining
    StartMine { pos: TilePos },
    /// Stop mining 
    StopMine,
    /// Request transfer to another world
    TransferRequest { target: WorldId },
    /// Disconnect
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerPacket {
    /// Sent ONLY on connection
    Welcome { id: u64 },
    /// Full world state for nearby players, broadcast every tick (to be updated)
    WorldSnapshot(WorldSnapshot),
    /// A player disconnected
    PlayerDisconnected { id: u64 },
    /// Tile mined — broadcast to nearby
    TileMined { pos: TilePos },
    /// Tile respawned — broadcast to nearby
    TileRespawned { pos: TilePos, tile_type: TileType },
    /// Mining progress update (only sent to the individual miner)
    MineProgress { pos: TilePos, progress: f32 },
    /// Mining cancelled
    MineCancelled,
    /// Successful approved
    TransferApproved { world: WorldId, spawn_x: f32, spawn_y: f32 },
    /// Transfer denied
    TransferDenied { reason: TransferError },
    /// Zone change
    ZoneChanged { zone: crate::zone::ZoneType },
    /// New player spawned
    PlayerSpawned(PlayerState),
}