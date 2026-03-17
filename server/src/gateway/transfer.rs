use shared_protocol::zone::WorldId;
use shared_protocol::world::TransferError;

use crate::gateway::route::SharedGateway;
use crate::world::instance::SharedInstance;

pub(crate) enum TransferResult {
    /// Transfer approved, spawned at (x, y)
    Approved {
        instance: SharedInstance,
        spawn_x: f32,
        spawn_y: f32,
    },
    /// Transfer denied with reason
    Denied(TransferError),
}

/// Attempt to transfer a player to a target world.
pub(crate) async fn try_transfer(
    player_id: u64,
    current_instance: &SharedInstance,
    target_world: &WorldId,
    shared_gateway: &SharedGateway,
) -> TransferResult {
    {
        let inst = current_instance.read().await;
        if let Some(player) = inst.players.get(&player_id) {
            if player.is_combat_tagged() {
                return TransferResult::Denied(TransferError::CombatTagged)
            }
        }
    }

    // Find target instance
    let gateway = shared_gateway.read().await;
    let target_instance = match gateway.get_instance(target_world) {
        Some(inst) => inst,
        None => {
            // Try to find any instance of that world
            match gateway.find_best_instance(&target_world.world_name) {
                Some(inst) => inst,
                None => {
                    return TransferResult::Denied(TransferError::WorldNotFound(target_world.world_name.clone()));
                }
            }
        }
    };
    drop(gateway);

    {
        let target = target_instance.read().await;
        if target.is_full() {
            return TransferResult::Denied(TransferError::WorldFull);
        }
    }

    let spawn_x = 0.0;
    let spawn_y = 0.0;

    TransferResult::Approved {
        instance: target_instance,
        spawn_x,
        spawn_y,
    }
}