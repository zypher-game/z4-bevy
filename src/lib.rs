use bevy::prelude::*;
use bevy_web3::{Contract, EthWallet, RecvError as Web3RecvError, Token, H160};
use serde::Deserialize;
use serde_json::{from_str, json, to_string, Value};
use z4_types::contracts::{Network, ROOM_MARKET_ABI};

pub type RoomId = u64;
pub use tdn_bevy::RecvError;
pub use tdn_types::{
    primitives::{PeerId, PeerKey},
    rpc::rpc_request,
};

#[cfg(feature = "wasm")]
use tdn_bevy::wasm::{HttpClient, HttpConnection, WasmClientPlugin};

pub const INIT_ROOM_MARKET_GROUP: RoomId = 100000;

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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WaitingRoom {
    pub room: u64,
    pub players: Vec<String>,
    pub sequencer: String,
    pub http: String,
}

/// Fetch pending rooms list from some node in RoomMarket
#[derive(Default, Resource)]
pub struct RoomMarket {
    pub contract: Contract,
    pub url: String,
    pub game: String,
    pub rooms: Vec<PendingRoom>,
    pub waiting: Option<WaitingRoom>,
}

/// Fetch pending rooms
#[cfg(feature = "wasm")]
pub fn fetch_room_market(
    mut commands: Commands,
    market: Res<RoomMarket>,
    http_client: Res<HttpClient>,
) {
    if !market.url.is_empty() && !market.game.is_empty() {
        http_client.jsonrpc(
            &mut commands,
            &market.url,
            "room_market",
            INIT_ROOM_MARKET_GROUP,
            json!(vec![&market.game]),
            vec![(
                "peer".to_owned(),
                json!("0x0000000000000000000000000000000000000000"),
            )],
        );
    }
}

/// Handle the room market response
#[cfg(feature = "wasm")]
pub fn handle_room_market(
    mut commands: Commands,
    mut market: ResMut<RoomMarket>,
    connections: Query<(Entity, &HttpConnection)>,
) {
    for (entity, connection) in &connections {
        if &connection.method == "room_market" {
            match connection.recv() {
                Ok(Ok(value)) => {
                    if let Ok(rooms) = serde_json::from_value::<Vec<PendingRoom>>(value) {
                        market.rooms = rooms;
                    }
                }
                Ok(Err(error)) => {
                    error!("Room Market ERROR: {}", error);
                    commands.entity(entity).despawn();
                }
                Err(RecvError::Empty) => {}
                Err(RecvError::Closed) => commands.entity(entity).despawn(),
            }
        }
    }
}

/// Fetch pending rooms
#[cfg(feature = "wasm")]
pub fn fetch_room_status(mut market: ResMut<RoomMarket>, wallet: Res<EthWallet>) {
    if market.waiting.is_some() {
        let room_id = market.waiting.as_ref().unwrap().room;
        // encode query
        if market.contract.is_empty() {
            let address = PeerId(
                Network::from_chain_id(wallet.chain_id)
                    .address("RoomMarket")
                    .unwrap(),
            );
            market.contract = Contract::load(&address.to_hex(), ROOM_MARKET_ABI.as_bytes());
        }

        let data = market
            .contract
            .encode("roomInfo", &[Token::Uint(room_id.into())]);
        wallet.call(market.contract.address, "roomInfo".to_owned(), data);
    }
}

/// Handle the room market response
#[cfg(feature = "wasm")]
pub fn handle_room_status(mut market: ResMut<RoomMarket>, wallet: Res<EthWallet>) {
    if market.waiting.is_some() {
        match wallet.recv_call() {
            Ok((method, bytes)) => {
                match method.as_str() {
                    "roomInfo" => {
                        let infos = market.contract.decode("roomInfo", &bytes);
                        info!("{:?}", infos);
                        // (address[] memory, address, address, uint256, RoomStatus)
                        // (room.players, room.game, room.sequencer, room.site, room.status)
                        let game = infos[1].clone().into_address().unwrap_or(H160::zero());
                        let sequencer = infos[2].clone().into_address().unwrap_or(H160::zero());
                        if sequencer != H160::zero() {
                            let seq = PeerId(sequencer.to_fixed_bytes()).to_hex();
                            if let Some(waiting) = &mut market.waiting {
                                waiting.sequencer = seq;
                            }

                            //  call sequencer info
                            let data = market.contract.encode(
                                "sequencers",
                                &[Token::Address(sequencer), Token::Address(game)],
                            );
                            wallet.call(market.contract.address, "sequencers".to_owned(), data);
                        }
                    }
                    "sequencers" => {
                        let infos = market.contract.decode("sequencers", &bytes);
                        info!("{:?}", infos);
                        let server = infos[0].clone().into_string().unwrap_or(String::new());
                        if let Some(waiting) = &mut market.waiting {
                            waiting.http = server;
                        }
                    }
                    _ => {
                        // TODO
                    }
                }
            }
            Err(Web3RecvError::Empty) => {}
            Err(Web3RecvError::Closed) => {}
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
            info!("{}", values);
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
