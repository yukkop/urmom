use std::net::UdpSocket;
use std::time::SystemTime;

use crate::actor::character::{spawn_character, spawn_tied_camera, TiedCamera};
use crate::actor::UnloadActorsEvent;
use crate::component::{DespawnReason, Respawn};
use crate::core::{KnownLevel};
use crate::lobby::{LobbyState, PlayerData, PlayerId, ServerMessages, Username};
use crate::world::{LinkId, Me, SpawnProperty};
use bevy::app::{App, Plugin, Update};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{Event, EventReader, EventWriter};
use bevy::ecs::query::With;
use bevy::ecs::schedule::{Condition, NextState, OnExit};
use bevy::ecs::system::{Query, Res, ResMut};
use bevy::hierarchy::DespawnRecursiveExt;

use bevy::prelude::{in_state, Color, Commands, IntoSystemConfigs, OnEnter};
use bevy_renet::transport::NetcodeServerPlugin;
use bevy_renet::RenetServerPlugin;
use renet::transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};

use super::{
    ChangeMapLobbyEvent, Character, HostResource, LevelCode, Lobby, MapLoaderState, TransportDataResource, PROTOCOL_ID,
};

#[derive(Debug, Event)]
pub struct DespawnActorEvent(pub LinkId);
#[derive(Debug, Event)]
pub struct SpawnProjectileEvent(pub LinkId, pub Color);

pub struct HostLobbyPlugins;

impl Plugin for HostLobbyPlugins {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnActorEvent>()
            .add_event::<SpawnProjectileEvent>()
            .add_plugins((RenetServerPlugin, NetcodeServerPlugin))
            .add_systems(OnEnter(LobbyState::Host), setup)
            .add_systems(
                Update,
                (send_change_map, spawn_projectile, despawn_actor)
                    .run_if(in_state(LobbyState::Host)),
            )
            .add_systems(
                Update,
                server_update_system.run_if(in_state(LobbyState::Host)),
            )
            .add_systems(OnExit(LobbyState::Host), teardown)
            .add_systems(
                Update,
                load_processing
                    .run_if(in_state(LobbyState::Host).and_then(in_state(MapLoaderState::No))),
            );
    }
}

pub fn spawn_projectile(
    mut event_reader: EventReader<SpawnProjectileEvent>,
    mut server: ResMut<RenetServer>,
) {
    for SpawnProjectileEvent(link_id, color) in event_reader.read() {
        let message = bincode::serialize(&ServerMessages::ProjectileSpawn {
            id: link_id.clone(),
            color: *color,
        })
        .unwrap();
        server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
}

pub fn despawn_actor(
    mut event_reader: EventReader<DespawnActorEvent>,
    mut server: ResMut<RenetServer>,
) {
    for DespawnActorEvent(link_id) in event_reader.read() {
        let message = bincode::serialize(&ServerMessages::ActorDespawn {
            id: link_id.clone(),
        })
        .unwrap();
        server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
}

pub fn new_renet_server(addr: &str) -> (RenetServer, NetcodeServerTransport) {
    let server = RenetServer::new(ConnectionConfig::default());

    let public_addr = addr.parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: 64,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

    (server, transport)
}

fn setup(
    mut commands: Commands,
    host_resource: Res<HostResource>,
    mut change_map_event: EventWriter<ChangeMapLobbyEvent>,
) {
    // resources for server
    commands.init_resource::<TransportDataResource>();
    commands.insert_resource(Lobby::default());

    // spanw server
    let (server, transport) = new_renet_server(host_resource.address.clone().unwrap().as_str());
    commands.insert_resource(server);
    commands.insert_resource(transport);

    change_map_event.send(ChangeMapLobbyEvent(LevelCode::Known(KnownLevel::Hub)));
}

pub fn load_processing(
    mut commands: Commands,
    spawn_point: Res<SpawnProperty>,
    mut lobby_res: ResMut<Lobby>,
    host_resource: Res<HostResource>,
    query: Query<(), With<Me>>,
    mut character_respawn_query: Query<&mut Respawn, With<Character>>,
    mut next_state_map: ResMut<NextState<MapLoaderState>>,
) {
    log::info!("LoadProcessing: {:#?}", spawn_point);
    if !spawn_point.is_empty() {
        if query.get_single().is_err() {
            // spawn host character
            lobby_res.players_seq += 1;
            let color = generate_player_color(lobby_res.players_seq as u32);

            let player_entity = commands
                .spawn_character(PlayerId::HostOrSingle, color, spawn_point.random_point())
                .insert(Me)
                .id();
            commands.spawn_tied_camera(player_entity);

            lobby_res.me = PlayerData::new(
                player_entity,
                color,
                host_resource.username.clone().unwrap(),
            );
        }

        for mut respawn in character_respawn_query.iter_mut() {
            respawn.replase_spawn_point(spawn_point.clone());
            respawn.insert_reason(DespawnReason::Forced);
        }

        next_state_map.set(MapLoaderState::Yes);
    }
}

pub fn send_change_map(
    mut change_map_event: EventReader<ChangeMapLobbyEvent>,
    mut server: ResMut<RenetServer>,
    // mut next_state_map: ResMut<NextState<MapState>>,
    mut unload_actors_event: EventWriter<UnloadActorsEvent>,
) {
    for ChangeMapLobbyEvent(_state) in change_map_event.read() {
        // next_state_map.set(*state);
        let message =
            bincode::serialize(&ServerMessages::ChangeMap { /*map_state: *state*/ }).unwrap();
        server.broadcast_message(DefaultChannel::ReliableOrdered, message);

        unload_actors_event.send(UnloadActorsEvent);
    }
}

fn teardown(
    mut commands: Commands,
    tied_camera_query: Query<Entity, With<TiedCamera>>,
    char_query: Query<Entity, With<Character>>,
    mut unload_actors_event: EventWriter<UnloadActorsEvent>,
) {
    for entity in tied_camera_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in char_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<Lobby>();
    commands.remove_resource::<TransportDataResource>();

    unload_actors_event.send(UnloadActorsEvent);
}

pub fn generate_player_color(player_number: u32) -> Color {
    let golden_angle = 137.5;
    let hue = (golden_angle * player_number as f32) % 360.0;
    Color::hsl(hue, 1.0, 0.5)
}

#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<Lobby>,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    spawn_point: Res<SpawnProperty>,
    //map_state: ResMut<State<MapState>>,

    //mut input_query: Query<&mut PlayerInputs>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                log::info!("Player {} connected.", client_id);

                // TODO remove
                let message = bincode::serialize(&ServerMessages::InitConnection {
                    id: *client_id,
                    //map_state: *map_state.get(),
                })
                .unwrap();
                server.send_message(*client_id, DefaultChannel::ReliableOrdered, message);

                lobby.players_seq += 1;
                let color = generate_player_color(lobby.players_seq as u32);

                // Spawn player cube
                let player_entity = commands
                    .spawn_character(
                        PlayerId::Client(*client_id),
                        color,
                        spawn_point.random_point(),
                    )
                    .id();

                // We could send an InitState with all the players id and positions for the multiplayer
                // but this is easier to do.
                for (player_id, player_data) in &lobby.players {
                    let message = bincode::serialize(&ServerMessages::PlayerConnected {
                        id: *player_id,
                        color: player_data.color,
                        username: player_data.username.clone(),
                    })
                    .unwrap();
                    server.send_message(*client_id, DefaultChannel::ReliableOrdered, message);
                }

                let data = transport.user_data(*client_id).unwrap();
                let username = match Username::from_user_data(&data) {
                    Ok(name) => name,
                    Err(_) => "@corapted@".to_string(),
                };
                // let username = "noname".to_string();

                lobby.players.insert(
                    PlayerId::Client(*client_id),
                    PlayerData::new(player_entity, color, username.clone()),
                );

                let message = bincode::serialize(&ServerMessages::PlayerConnected {
                    id: PlayerId::Client(*client_id),
                    color,
                    username,
                })
                .unwrap();
                server.broadcast_message(DefaultChannel::ReliableOrdered, message);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                log::info!("Player {} disconnected: {}", client_id, reason);
                if let Some(player_data) = lobby.players.remove(&PlayerId::Client(*client_id)) {
                    commands.entity(player_data.entity()).despawn();
                }

                let message = bincode::serialize(&ServerMessages::PlayerDisconnected {
                    id: PlayerId::Client(*client_id),
                })
                .unwrap();
                server.broadcast_message(DefaultChannel::ReliableOrdered, message);
            }
        }
    }

    for client_id in server.clients_id().into_iter() {
        let _first = true;
        while let Some(_message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            // let input: Inputs = bincode::deserialize(&message).unwrap();
            if let Some(_player_data) = lobby.players.get(&PlayerId::Client(client_id)) {
                // TODO:
                // if let Ok(mut player_input) = input_query.get_mut(player_data.entity()) {
                //     if first {
                //         player_input.insert_inputs(input);
                //         first = false;
                //     } else {
                //         player_input.add(input);
                //     }
                // }
            } else {
                log::error!("Player not found");
            }
        }
    }
}

// pub fn server_sync_actor(
//     mut server: ResMut<RenetServer>,
//     // TODO a nahooya tut resours, daun
//     mut data: ResMut<TransportDataResource>,
//     character_query: Query<(&Position, &Rotation, &PlayerView, &Character)>,
//     moveble_actor_query: Query<(&Transform, &LinkId)>,
// ) {
//     let data = &mut data.data;
//     for (position, rotation, view_direction, character) in character_query.iter() {
//         data.players.insert(
//             character.id,
//             PlayerTransportData {
//                 position: position.0,
//                 rotation: rotation.0,
//                 player_view: *view_direction,
//             },
//         );
//     }
//
//     for (transform, link_id) in moveble_actor_query.iter() {
//         data.actors.insert(
//             link_id.clone(),
//             ActorTransportData {
//                 position: transform.translation,
//                 rotation: transform.rotation,
//             },
//         );
//     }
//
//     let sync_message = bincode::serialize(&data).unwrap();
//     server.broadcast_message(DefaultChannel::Unreliable, sync_message);
//
//     data.players.clear();
//     data.actors.clear();
// }
