use bevy::prelude::*;
use z4_bevy::{
    ws::{ws_connect, WsConnection},
    PeerKey, RecvError, Z4ClientPlugin,
};

const MY_ROOM_ID: u64 = 1;
const MY_SERVER: &str = "ws://127.0.0.1:8000";

#[derive(Component)]
struct Name(String);

fn main() {
    App::new()
        //.add_plugins(DefaultPlugins)
        .add_plugins(Z4ClientPlugin)
        .add_systems(Startup, init_ws_connect)
        .add_systems(Update, ws_receive)
        .run();
}

fn init_ws_connect(mut commands: Commands) {
    let peer_key = PeerKey::default();
    println!("=====");
    ws_connect(&mut commands, MY_SERVER, &peer_key, MY_ROOM_ID);
}

fn ws_receive(mut commands: Commands, connections: Query<(Entity, &WsConnection)>) {
    for (entity, conn) in connections.iter() {
        match conn.recv() {
            Ok(message) => {
                println!("message: {}", message);
                // conn.send(message);
            }
            Err(RecvError::Empty) => break,
            Err(RecvError::Closed) => {
                commands.entity(entity).despawn();
                break;
            }
        }
    }
}
