use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{from_str, json, to_string, Value};

pub type RoomId = u64;
pub use tdn_bevy::RecvError;
pub use tdn_types::{primitives::PeerKey, rpc::rpc_request};

#[cfg(feature = "wasm")]
use tdn_bevy::wasm::{HttpClient, HttpConnection, WasmClientPlugin};

pub struct Z4ClientPlugin;

impl Plugin for Z4ClientPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "ws")]
        app.add_plugins(tdn_bevy::ws::WsClientPlugin);

        #[cfg(feature = "wasm")]
        app.add_plugins(WasmClientPlugin);

        app.insert_resource(RoomMarket::default());
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PendingRoom {
    pub room: u64,
    pub players: Vec<String>,
}

/// Fetch pending rooms list from some node in RoomMarket
#[derive(Default, Resource)]
pub struct RoomMarket {
    pub url: String,
    pub game: String,
    pub rooms: Vec<PendingRoom>,
}

/// Fetch pending rooms
pub fn fetch_room_market(
    mut commands: Commands,
    list: Res<RoomMarket>,
    http_client: Res<HttpClient>,
) {
    if !list.url.is_empty() && !list.game.is_empty() {
        http_client.jsonrpc(
            &mut commands,
            &list.url,
            "room_market",
            0, // gid
            json!(vec![&list.game]),
            vec![(
                "peer".to_owned(),
                json!("0x0000000000000000000000000000000000000000"),
            )],
        );
    }
}

/// Handle the room market response
pub fn handle_room_market(
    mut commands: Commands,
    mut list: ResMut<RoomMarket>,
    connections: Query<(Entity, &HttpConnection)>,
) {
    for (entity, connection) in &connections {
        if &connection.method == "room_market" {
            match connection.recv() {
                Ok(Ok(value)) => {
                    if let Ok(rooms) = serde_json::from_value::<Vec<PendingRoom>>(value) {
                        list.rooms = rooms;
                    }
                }
                Ok(Err(error)) => {
                    error!("Room Market ERROR: {}", error);
                    commands.entity(entity).despawn()
                }
                Err(RecvError::Empty) => {}
                Err(RecvError::Closed) => commands.entity(entity).despawn(),
            }
        }
    }
}

#[cfg(any(feature = "ws", feature = "wasm"))]
pub use tdn_bevy::Message;

#[cfg(any(feature = "ws", feature = "wasm"))]
#[inline]
pub fn build_request(method: &str, v: Vec<Value>, peer: &PeerKey, room: RoomId) -> Message {
    let mut request = rpc_request(0, &method, v, room);
    request
        .as_object_mut()
        .unwrap()
        .insert("peer".to_owned(), peer.peer_id().to_hex().into());
    Message::from(to_string(&request).unwrap_or("".to_owned()))
}

#[cfg(any(feature = "ws", feature = "wasm"))]
#[inline]
pub fn parse_response(msg: &Message) -> Result<(RoomId, String, Vec<Value>), String> {
    let msg = msg.to_text().unwrap_or("");
    match from_str::<Value>(&msg) {
        Ok(mut values) => {
            let gid = values["gid"].as_u64().unwrap(); // TODO unwrap
            let method = values["method"].as_str().unwrap().to_owned();
            // let server_id = values["peer"].as_str().unwrap(); TODO
            let tmp = values["result"].take().as_array().unwrap().to_vec();
            return Ok((gid, method, tmp));
        }
        Err(_e) => {}
    }
    Err(String::from("Invalid"))
}

#[cfg(feature = "ws")]
pub mod ws {
    use super::*;
    pub use tdn_bevy::ws::{WsClient, WsConnection};

    #[inline]
    pub fn ws_connect(commands: &mut Commands, url: &str, peer: &PeerKey, room: RoomId) {
        // build the z4 connect init message
        let msg = build_request("connect", vec![], peer, room);
        WsClient.connect(commands, url, Some(msg));
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use super::*;
    pub use tdn_bevy::wasm::{WsClient, WsConnection};

    #[inline]
    pub fn ws_connect(commands: &mut Commands, url: &str, peer: &PeerKey, room: RoomId) {
        // build the z4 connect init message
        let msg = build_request("connect", vec![], peer, room);
        WsClient.connect(commands, url, Some(msg));
    }
}
