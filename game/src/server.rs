use crate::{
    actor::ActorKind,
    level::Level,
    net::{
        ClientMessage, InstanceDescriptor, LeaderBoardMessage, NodeState, PlayerDescriptor,
        ServerMessage, SoundState, UpdateTickMessage,
    },
    player::Player,
    start::StartPoint,
};
use fyrox::graph::SceneGraphNode;
use fyrox::plugin::error::GameResult;
use fyrox::scene::graph::GraphError;
use fyrox::{
    core::{
        futures::executor::block_on,
        log::Log,
        net::{NetListener, NetStream},
        pool::Handle,
    },
    fxhash::FxHashMap,
    graph::SceneGraph,
    plugin::PluginContext,
    resource::model::{Model, ModelResourceExtension},
    scene::{
        node::Node,
        sound::{Sound, Status},
        Scene,
    },
};
use std::fmt::{Debug, Formatter};
use std::net::SocketAddr;
use std::ops::Deref;
use std::{io, net::ToSocketAddrs, path::Path};

pub struct Server {
    listener: NetListener,
    connections: Vec<NetStream>,
    previous_node_states: FxHashMap<Handle<Node>, NodeState>,
    previous_sound_states: FxHashMap<Handle<Node>, SoundState>,
    pub add_bots: bool,
}

impl Debug for Server {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server")
    }
}

impl Server {
    pub const LOCALHOST: &'static str = "127.0.0.1:10001";

    pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Ok(Self {
            listener: NetListener::bind(addr)?,
            connections: Default::default(),
            previous_node_states: Default::default(),
            previous_sound_states: Default::default(),
            add_bots: true,
        })
    }

    pub fn broadcast_message_to_clients(&mut self, message: ServerMessage) {
        for client_connection in self.connections.iter_mut() {
            match client_connection.send_message(&message) {
                Ok(_) => {}
                Err(err) => Log::err(format!("Unable to send server message: {}", err)),
            }
        }
    }

    pub fn start_game(&mut self, path: &Path) {
        self.broadcast_message_to_clients(ServerMessage::LoadLevel {
            path: path.to_path_buf(),
        });
    }

    pub fn update(&mut self, level: &mut Level, ctx: &mut PluginContext) -> GameResult {
        level.update(ctx)?;

        if let Ok(scene) = ctx.scenes.try_get_mut(level.scene) {
            if level.is_match_ended() {
                self.broadcast_message_to_clients(ServerMessage::EndMatch);
            }

            self.broadcast_message_to_clients(ServerMessage::LeaderBoard(LeaderBoardMessage {
                players: level.leaderboard.entries.values().cloned().collect(),
            }));

            let mut tick_data = UpdateTickMessage {
                nodes: Default::default(),
                sounds: Default::default(),
            };

            for (handle, node) in scene.graph.pair_iter() {
                let current_state = NodeState {
                    node: node.deref().instance_id(),
                    position: **node.local_transform().position(),
                    rotation: **node.local_transform().rotation(),
                };

                // Dead simple delta compression.
                let prev_state = self
                    .previous_node_states
                    .entry(handle)
                    .or_insert(current_state.clone());

                if *prev_state != current_state {
                    tick_data.nodes.push(current_state.clone());
                    *prev_state = current_state;
                }

                if let Some(sound) = node.component_ref::<Sound>() {
                    let current_state = SoundState {
                        node: sound.instance_id(),
                        is_playing: sound.status() == Status::Playing,
                    };

                    let prev_state = self
                        .previous_sound_states
                        .entry(handle)
                        .or_insert(current_state.clone());

                    if *prev_state != current_state {
                        tick_data.sounds.push(current_state.clone());
                        *prev_state = current_state;
                    }
                }
            }

            self.broadcast_message_to_clients(ServerMessage::UpdateTick(tick_data));
        }

        Ok(())
    }

    pub fn read_messages(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) -> GameResult {
        for player in self.connections.iter_mut() {
            while let Some(msg) = player.pop_message::<ClientMessage>() {
                match msg {
                    ClientMessage::Input {
                        player,
                        input_state,
                    } => {
                        let scene = &mut ctx.scenes[scene];
                        let (_, player_node) = scene.graph.node_by_id_mut(player)?;
                        player_node
                            .try_get_script_mut::<Player>()
                            .ok_or_else(|| GraphError::NoScript {
                                handle: Default::default(),
                                script_type_name: std::any::type_name::<Player>(),
                            })?
                            .input_controller = input_state;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn on_scene_loaded(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        let scene = &mut ctx.scenes[scene];
        let players_to_spawn = self.connections.len();

        let start_points = scene
            .graph
            .linear_iter()
            .filter(|n| n.has_script::<StartPoint>())
            .map(|n| n.global_position())
            .collect::<Vec<_>>();

        let player_prefab = block_on(
            ctx.resource_manager
                .request::<Model>("data/models/player.rgs"),
        )
        .unwrap();

        for player_num in 0..players_to_spawn {
            let ids = player_prefab.generate_ids();

            if let Some(position) = start_points.get(player_num) {
                for (connection_num, connection) in self.connections.iter_mut().enumerate() {
                    connection
                        .send_message(&ServerMessage::AddPlayers(vec![PlayerDescriptor {
                            instance: InstanceDescriptor {
                                path: "data/models/player.rgs".into(),
                                position: *position,
                                rotation: Default::default(),
                                velocity: Default::default(),
                                ids: ids.clone(),
                            },
                            kind: if player_num != connection_num {
                                ActorKind::RemotePlayer
                            } else {
                                ActorKind::Player
                            },
                        }]))
                        .unwrap();
                }
            }
        }

        if self.add_bots {
            let bot_prefab =
                block_on(ctx.resource_manager.request::<Model>("data/models/bot.rgs")).unwrap();

            for i in players_to_spawn..start_points.len() {
                let ids = bot_prefab.generate_ids();

                if let Some(position) = start_points.get(i) {
                    for connection in self.connections.iter_mut() {
                        connection
                            .send_message(&ServerMessage::AddPlayers(vec![PlayerDescriptor {
                                instance: InstanceDescriptor {
                                    path: "data/models/bot.rgs".into(),
                                    position: *position,
                                    rotation: Default::default(),
                                    velocity: Default::default(),
                                    ids: ids.clone(),
                                },
                                kind: ActorKind::Bot,
                            }]))
                            .unwrap();
                    }
                }
            }
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.listener.local_address().unwrap()
    }

    pub fn connections(&self) -> &[NetStream] {
        &self.connections
    }

    pub fn is_single_player(&self) -> bool {
        self.connections.len() == 1
    }

    pub fn accept_connections(&mut self) {
        self.connections.extend(self.listener.accept_connections())
    }
}
