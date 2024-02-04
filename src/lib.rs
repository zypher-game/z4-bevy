use bevy::prelude::*;
use tdn_bevy::{RecvError, WsClient, WsClientPlugin, WsConnection};

pub struct Z4ClientPlugin;

impl Plugin for Z4ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WsClientPlugin);
    }
}

fn connect_ws(mut commands: Commands, ws_client: Res<WsClient>) {
    ws_client.connect(&mut commands, "127.0.0.1:8000", None);
}

fn receive_message(mut commands: Commands, connections: Query<(Entity, &WsConnection)>) {
    for (entity, conn) in connections.iter() {
        loop {
            match conn.recv() {
                Ok(message) => {
                    println!("message: {}", message);
                    conn.send(message);
                }
                Err(RecvError::Empty) => break,
                Err(RecvError::Closed) => {
                    commands.entity(entity).despawn();
                    break;
                }
            }
        }
    }
}
