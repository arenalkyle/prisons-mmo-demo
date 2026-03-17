use std::sync::Arc;
use std::time::{Duration, Instant};

use shared_protocol::framing::{read_frame, write_frame};
use shared_protocol::movement::MovementInput;
use shared_protocol::packets::{ClientPacket, ServerPacket};
use shared_protocol::zone::WorldId;
use shared_protocol::world::TransferError;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::gateway::route::SharedGateway;
use crate::gateway::transfer;
use crate::player::server_player::ServerPlayer;
use crate::world::instance::SharedInstance;

const MAX_CLIENT_QUEUE: usize = 64;
const TICK_RATE: u64 = 20;
const TICK_DURATION: Duration = Duration::from_millis(1000 / TICK_RATE);

/// Run a tick loop for a specific world instance.
pub(crate) async fn instance_tick_loop(instance: SharedInstance) {
    let mut interval = tokio::time::interval(TICK_DURATION);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_status = Instant::now();

    loop {
        interval.tick().await;

        let mut inst = instance.write().await;
        inst.tick();

        let count = inst.player_count();
        let id = inst.id.clone();
        drop(inst);

        if last_status.elapsed() >= Duration::from_secs(10) && count > 0 {
            println!("[{}] tick — {} players", id, count);
            last_status = Instant::now();
        }
    }
}

/// Handle a connected client. Routes them to a default world, then processes packets.
pub(crate) async fn handle_client(
    mut socket: TcpStream,
    player_id: u64,
    gateway: SharedGateway,
    default_world: WorldId,
) {
    let (tx, mut rx) = mpsc::channel::<Arc<Vec<u8>>>(MAX_CLIENT_QUEUE);

    // Find the default instance
    let instance = {
        let gw = gateway.read().await;
        match gw.get_instance(&default_world) {
            Some(inst) => inst,
            None => {
                eprintln!("[server] No instance for {:?}, dropping client {}", default_world, player_id);
                return;
            }
        }
    };

    let spawn_x = 0.0;
    let spawn_y = 0.0;

    let player = ServerPlayer::new(player_id, tx, default_world.clone());

    // Add to instance
    {
        let mut inst = instance.write().await;
        if !inst.add_player(player, spawn_x, spawn_y) {
            eprintln!("[server] Instance {} full, rejecting player {}", default_world, player_id);
            let reject = ServerPacket::TransferDenied {
                reason: TransferError::WorldFull,
            };

            let bytes = bincode::serialize(&reject).unwrap();
            let _ = write_frame(&mut socket, &bytes).await;
            return;
        }
    }

    // Send Welcome
    let welcome = ServerPacket::Welcome { id: player_id };
    let bytes = bincode::serialize(&welcome).unwrap();
    if write_frame(&mut socket, &bytes).await.is_err() {
        instance.write().await.remove_player(player_id);
        return;
    }
    let (mut reader, mut writer) = socket.into_split();

    let write_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {

            // Simulate artificial lag -> 50ms delay for every outgoing packet.
            // Uncomment line 100 to simulate.
            // tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            let len = (data.len() as u32).to_le_bytes();
            if writer.write_all(&len).await.is_err() || writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    // Current instance tracking (changes on transfer)
    let mut current_instance = instance;

    let mut buf = Vec::new();
    let mut last_input_time = Instant::now();
    let min_input_interval = Duration::from_millis(8);

    loop {
        let frame = read_frame(&mut reader, &mut buf).await;

        match frame {
            Ok(Some(len)) => {
                let packet_slice = &buf[..len];

                match bincode::deserialize::<ClientPacket>(&packet_slice) {
                    Ok(packet) => {
                        let should_break = handle_packet(
                            player_id,
                            packet,
                            &mut current_instance,
                            &gateway,
                            &mut last_input_time,
                            min_input_interval,
                        )
                            .await;
                        if should_break {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[server] Bad packet from {}: {}", player_id, e);
                        break;
                    }
                }
            }

            Ok(None) => {
                println!("[server] Client {} disconnected cleanly", player_id);
                break;
            }
            Err(e) => {
                eprintln!("[server] Read error from {}: {}", player_id, e);
                break;
            }
        }
    }

    current_instance.write().await.remove_player(player_id);
    write_task.abort();
    println!("[server] Client {} fully cleaned up", player_id);
}

/// Returns true if the client should disconnect.
async fn handle_packet(
    player_id: u64,
    packet: ClientPacket,
    current_instance: &mut SharedInstance,
    gateway: &SharedGateway,
    last_input_time: &mut Instant,
    min_input_interval: Duration,
) -> bool {
    match packet {
        ClientPacket::MoveInput(input) => {
            let now = Instant::now();
            if now.duration_since(*last_input_time) < min_input_interval {
                return false;
            }
            *last_input_time = now;

            if !validate_input(&input) {
                return false;
            }

            current_instance.write().await.apply_input(player_id, &input);
            false
        }

        ClientPacket::StartMine { pos } => {
            current_instance.write().await.start_mining(player_id, pos);
            false
        }

        ClientPacket::StopMine => {
            current_instance.write().await.stop_mining(player_id);
            false
        }

        ClientPacket::TransferRequest { target } => {
            let result = transfer::try_transfer(
                player_id,
                current_instance,
                &target,
                gateway,
            )
                .await;

            match result {
                transfer::TransferResult::Approved {
                    instance: new_instance,
                    spawn_x,
                    spawn_y,
                } => {
                    // Extract player from old instance (keeps tx alive)
                    let mut old_inst = current_instance.write().await;
                    let mut player = match old_inst.take_player(player_id) {
                        Some(p) => p,
                        None => return false,
                    };
                    drop(old_inst);

                    let pkt = ServerPacket::TransferApproved {
                        world: target,
                        spawn_x,
                        spawn_y,
                    };

                    let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                    player.send(&bytes);

                    player.world_id = new_instance.read().await.id.clone();
                    player.mining = None;

                    // Add to a new instance
                    let mut new_inst = new_instance.write().await;
                    if !new_inst.add_player(player, spawn_x, spawn_y) {
                        // Instance became full between check and now — this is a safety precaution.
                        eprintln!("[server] Transfer race: {} lost, disconnecting", player_id);
                        return true;
                    }
                    drop(new_inst);

                    *current_instance = new_instance;
                }

                transfer::TransferResult::Denied(reason) => {
                    let pkt = ServerPacket::TransferDenied { reason };
                    let bytes = Arc::new(bincode::serialize(&pkt).unwrap());
                    let inst = current_instance.read().await;
                    if let Some(player) = inst.players.get(&player_id) {
                        player.send(&bytes);
                    }
                }
            }
            false
        }

        ClientPacket::Disconnect => {
            println!("[server] Client {} sent Disconnect", player_id);
            true
        }
    }
}

fn validate_input(input: &MovementInput) -> bool {
    if input.dir_x.abs() > 1.01 || input.dir_y.abs() > 1.01 {
        return false;
    }
    if input.dt < 0.0 || input.dt > 0.2 {
        return false;
    }
    true
}