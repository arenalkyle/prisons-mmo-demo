mod gateway;
mod network;
mod player;
mod world;

use std::sync::Arc;

use shared_protocol::tile::{TilePos, TileType};
use shared_protocol::zone::WorldId;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use gateway::route::{Gateway, SharedGateway};
use world::instance::WorldInstance;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:7777").await.unwrap();
    println!("[server] Listening on 0.0.0.0:7777");

    // Create gateway
    let shared_gateway: SharedGateway = Arc::new(RwLock::new(Gateway::new()));

    // Create world instances
    let default_world = WorldId::new("cells_hub", 1);
    let kremwood_1 = WorldId::new("kremwood_1", 1);
    let kremwood_2 = WorldId::new("kremwood_1", 2);

    {
        let mut gw = shared_gateway.write().await;

        // Cells Hub
        gw.register_instance(WorldInstance::new(default_world.clone(), 500));

        // Kremwood Forest instances
        let mut kf1 = WorldInstance::new(kremwood_1.clone(), 300);
        let kf2 = WorldInstance::new(kremwood_2.clone(), 300);
        
        // Sample mine-able tiles
        for x in 0..20 {
            for y in 0..20 {
                kf1.tiles.add_tile(TilePos::new(x, y), TileType::Stone);
            }
        }
        kf1.tiles.add_tile(TilePos::new(10, 10), TileType::Wood);
        
        gw.register_instance(kf1);
        gw.register_instance(kf2);

        println!("[server] Registered instances: {:?}", gw.list_instances());
    }

    {
        let gateway = shared_gateway.read().await;
        for world_id in gateway.list_instances() {
            if let Some(instance) = gateway.get_instance(&world_id) {
                let wid_display = world_id.to_string(); 
                tokio::spawn(async move {
                    println!("[server] Tick loop started for {}", wid_display);
                    network::logic::instance_tick_loop(instance).await;
                });
            }
        }
    }

    let mut next_id: u64 = 1;

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("[server] Connection from {}", addr);
        let id = next_id;
        next_id += 1;

        let gw = shared_gateway.clone();
        let default = default_world.clone();
        tokio::spawn(network::logic::handle_client(socket, id, gw, default));
    }
}