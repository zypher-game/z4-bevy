use bevy::prelude::*;
use serde_json::{from_str, to_string, Value};
use tdn::types::rpc::rpc_request;
use tdn_bevy::{Message, WsClientPlugin};

pub type RoomId = u64;

pub use tdn::prelude::PeerKey;
pub use tdn_bevy::{RecvError, WsClient, WsConnection};

pub struct Z4ClientPlugin;

impl Plugin for Z4ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WsClientPlugin);
    }
}

#[inline]
pub fn ws_connect(commands: &mut Commands, url: &str, peer: &PeerKey, room: RoomId) {
    // build the z4 connect init message
    let msg = build_request("connect", vec![], peer, room);
    WsClient.connect(commands, url, Some(msg));
}

#[inline]
pub fn build_request(method: &str, v: Vec<Value>, peer: &PeerKey, room: RoomId) -> Message {
    let mut request = rpc_request(0, &method, v, room);
    request
        .as_object_mut()
        .unwrap()
        .insert("peer".to_owned(), peer.peer_id().to_hex().into());
    Message::from(to_string(&request).unwrap_or("".to_owned()))
}

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
