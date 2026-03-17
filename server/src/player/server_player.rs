use std::sync::Arc;
use std::time::Instant;
use shared_protocol::player::PlayerState;
use shared_protocol::tile::TilePos;
use shared_protocol::zone::{WorldId, ZoneType};
use tokio::sync::mpsc;

/// Server-side player with full state (combat tags, mining, zone tracking).
pub(crate) struct ServerPlayer {
    /// Network state shared with the client
    pub state: PlayerState,
    /// Outbound message channel
    pub tx: mpsc::Sender<Arc<Vec<u8>>>,
    /// Which world instance this player is in
    pub world_id: WorldId,
    /// Current zone type the player is standing in
    pub zone: ZoneType,
    /// Combat tag — when set, player cannot transfer or safely disconnect
    pub combat_tag_until: Option<Instant>,
    /// Active mining state
    pub mining: Option<MiningState>,
}

pub(crate) struct MiningState {
    pub pos: TilePos,
    pub ticks_remaining: u32,
}

impl ServerPlayer {
    pub fn new(id: u64, tx: mpsc::Sender<Arc<Vec<u8>>>, world_id: WorldId) -> Self {
        Self {
            state: PlayerState::new(id),
            tx,
            world_id,
            zone: ZoneType::Safe,
            combat_tag_until: None,
            mining: None,
        }
    }

    pub fn is_combat_tagged(&self) -> bool {
        self.combat_tag_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    pub fn set_combat_tag(&mut self, duration_secs: f32) {
        self.combat_tag_until = Some(Instant::now() + std::time::Duration::from_secs_f32(duration_secs));
    }

    pub fn clear_combat_tag(&mut self) {
        self.combat_tag_until = None;
    }

    /// Send data to this player. Returns false if the channel is full (slow client).
    pub fn send(&self, data: &Arc<Vec<u8>>) -> bool {
        self.tx.try_send(Arc::clone(data)).is_ok()
    }
}