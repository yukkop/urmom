use crate::core::{CoreAction, KnownLevel};
use crate::world::LinkId;
use bevy::app::{App, Plugin};
use bevy::ecs::event::Event;
use bevy::math::{Quat, Vec3};
use bevy::prelude::{Color, Component, Entity, Resource, States};
use bevy::reflect::Reflect;
use bevy_controls::contract::InputsContainer;
use bevy_controls::resource::PlayerActions;
use renet::transport::NETCODE_USER_DATA_BYTES;
use renet::ClientId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::client::ClientLobbyPlugins;
use super::host::HostLobbyPlugins;
use super::single::SingleLobbyPlugins;

//use super::host::HostLobbyPlugins;
//use super::single::SingleLobbyPlugins;

pub const PROTOCOL_ID: u64 = 7;

/// An enumeration representing the states of a lobby system.
///
/// The [`LobbyState`] enum is used to define the various states that a lobby system can be in.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum LobbyState {
    /// Indicates that the lobby system is in no specific state (default state).
    #[default]
    None = 0,
    /// Represents the state where a single player is present in the lobby.
    Single = 1,
    /// Represents the state where a player is hosting the lobby.
    Host = 2,
    /// Represents the state where a player is a client in the lobby.
    Client = 3,
}

/// Represents different types of messages that a server can send.
///
/// This enum is used to encapsulate various messages that a server
/// in a multiplayer game may need to send.
/// Each variant of the enum represents a different type of message
/// with its own associated data.
#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    /// Sent when initializing a connection with a client.
    ///
    /// This message includes the client's ID and their initial map state.
    ///
    /// # Fields
    ///
    /// * `id` - Unique identifier for the connecting client.
    /// * `map_state` - Initial state of the client's map.
    InitConnection {
        id: ClientId,
        //map_state: MapState,
    },
    /// Sent to notify a change in the map's state.
    ///
    /// # Fields
    ///
    /// * `map_state` - The new state of the map.
    ChangeMap {
        //map_state: MapState,
    },
    /// Indicates that a player has connected to the server.
    ///
    /// # Fields
    ///
    /// * `id` - Unique identifier for the player.
    /// * `color` - The color assigned to the player.
    /// * `username` - The player's chosen username.
    PlayerConnected {
        id: PlayerId,
        color: Color,
        username: String,
    },
    /// Indicates that a player has disconnected from the server.
    ///
    /// # Fields
    ///
    /// * `id` - Unique identifier for the player who has disconnected.
    PlayerDisconnected {
        id: PlayerId,
    },
    ProjectileSpawn {
        id: LinkId,
        color: Color,
    },
    ActorDespawn {
        id: LinkId,
    },
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum MapLoaderState {
    Yes,
    #[default]
    No,
}

#[derive(Resource)]
pub struct Username(pub String);

impl Default for Username {
    fn default() -> Self {
        Self("noname".to_string())
    }
}

impl Username {
    pub fn to_netcode_data(
        &self,
    ) -> Result<[u8; NETCODE_USER_DATA_BYTES], Box<dyn std::error::Error>> {
        let mut data = [0u8; NETCODE_USER_DATA_BYTES];
        if self.0.len() > NETCODE_USER_DATA_BYTES - 8 {
            let err = Err(From::from("Your username to long"));
            log::error!("{:?}", err);
            return err;
        }
        data[0..8].copy_from_slice(&(self.0.len() as u64).to_le_bytes());
        data[8..self.0.len() + 8].copy_from_slice(self.0.as_bytes());

        Ok(data)
    }

    pub fn from_user_data(
        user_data: &[u8; NETCODE_USER_DATA_BYTES],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(&user_data[0..8]);
        let mut len = u64::from_le_bytes(buffer) as usize;
        len = len.min(NETCODE_USER_DATA_BYTES - 8);
        let data = user_data[8..len + 8].to_vec();
        let username = String::from_utf8(data)?;

        Ok(username)
    }
}

#[derive(Debug, Default, Resource)]
pub struct ClientResource {
    pub address: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Default, Resource)]
pub struct HostResource {
    pub address: Option<String>,
    pub username: Option<String>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct Lobby {
    // When the game does not provide multiplayer, one field is enough
    pub me: PlayerData,
    pub players: HashMap<PlayerId, PlayerData>,
    pub players_seq: usize,
}

impl InputsContainer<CoreAction> for Lobby {
    fn iter_inputs<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PlayerActions<CoreAction>> + 'a> {
        todo!()
    }

    fn me(&self) -> Option<&PlayerActions<CoreAction>> {
        Some(&self.me.inputs)
    }

    fn me_mut(&mut self) -> Option<&mut PlayerActions<CoreAction>> {
        Some(&mut self.me.inputs)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Default)]
pub enum PlayerId {
    #[default]
    HostOrSingle, // TODO: depricated
    Client(ClientId),
}

impl PlayerId {
    // TODO: forgoten realization
    #[allow(dead_code)]
    pub fn client_id(&self) -> Option<ClientId> {
        match self {
            PlayerId::HostOrSingle => None,
            PlayerId::Client(id) => Some(*id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerData {
    entity: Option<Entity>,
    pub color: Color,
    pub username: String,
    pub inputs: PlayerActions<CoreAction>,
}

impl PlayerData {
    pub fn new(entity: Entity, color: Color, username: String) -> PlayerData {
        PlayerData {
            entity: Some(entity),
            color,
            username,
            inputs: PlayerActions::<CoreAction>::default(),
        }
    }

    pub fn entity(&self) -> Entity {
        match self.entity {
            Some(entity) => entity,
            None => panic!(),
        }
    }
}

impl Default for PlayerData {
    fn default() -> Self {
        PlayerData {
            entity: None,
            color: Color::RED,
            username: "noname".into(),
            inputs: PlayerActions::<CoreAction>::default(),
        }
    }
}

#[derive(Debug, Component)]
pub struct Character {
    pub id: PlayerId,
}

#[derive(Resource, Default, Debug, Serialize, Deserialize)]
pub struct PlayerTransportData {
    pub position: Vec3,
    pub rotation: Quat,
    pub player_view: PlayerView,
}

#[derive(Resource, Default, Debug, Serialize, Deserialize)]
pub struct ActorTransportData {
    pub position: Vec3,
    pub rotation: Quat,
}

#[derive(Resource, Default, Debug, Serialize, Deserialize)]
pub struct TransportData {
    pub players: HashMap<PlayerId, PlayerTransportData>,
    pub actors: HashMap<LinkId, ActorTransportData>,
}

#[derive(Resource, Default, Debug, Serialize, Deserialize)]
pub struct TransportDataResource {
    pub data: TransportData,
}

#[derive(Debug, Component, Default, Serialize, Deserialize, Clone, Copy, Reflect)]
pub struct PlayerView {
    pub direction: Quat,
    pub distance: f32,
}

impl PlayerView {
    pub fn new(direction: Quat, distance: f32) -> Self {
        Self {
            direction,
            distance,
        }
    }
}

// TODO: to core.rs
#[derive(Debug, Clone)]
pub enum LevelCode {
    Url(String),
    Path(String),
    Known(KnownLevel),
}

#[derive(Debug, Event)]
pub struct ChangeMapLobbyEvent(pub LevelCode);

pub struct LobbyPlugins;

impl Plugin for LobbyPlugins {
    fn build(&self, app: &mut App) {
        app.add_event::<ChangeMapLobbyEvent>()
            .insert_state(LobbyState::default())
            .insert_state(MapLoaderState::default())
            .init_resource::<HostResource>()
            .init_resource::<ClientResource>()
            .add_plugins((HostLobbyPlugins, SingleLobbyPlugins, ClientLobbyPlugins));
    }
}
