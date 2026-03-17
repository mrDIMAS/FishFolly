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
use fyrox::core::info;
use fyrox::{
    core::{
        futures::executor::block_on,
        log::Log,
        net::{NetListener, NetStream},
        pool::Handle,
    },
    fxhash::FxHashMap,
    graph::{SceneGraph, SceneGraphNode},
    plugin::{error::GameResult, PluginContext},
    resource::model::{Model, ModelResourceExtension},
    scene::{
        graph::GraphError,
        node::Node,
        sound::{Sound, Status},
        Scene,
    },
};
use std::{
    fmt::{Debug, Formatter},
    io,
    net::{SocketAddr, ToSocketAddrs},
    ops::Deref,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
};

pub enum ServerTransportLayer {
    Memory {
        clients: Vec<Sender<ServerMessage>>,
        receiver: Receiver<ClientMessage>,
        sender: Sender<ClientMessage>,
    },
    Tcp {
        listener: NetListener,
        connections: Vec<NetStream>,
    },
}

impl ServerTransportLayer {
    fn memory() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self::Memory {
            clients: Default::default(),
            receiver,
            sender,
        }
    }

    fn tcp<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Ok(Self::Tcp {
            listener: NetListener::bind(addr)?,
            connections: Default::default(),
        })
    }

    fn num_connections(&self) -> usize {
        match self {
            ServerTransportLayer::Memory { clients, .. } => clients.len(),
            ServerTransportLayer::Tcp { connections, .. } => connections.len(),
        }
    }

    fn broadcast_message_to_clients_callback<C>(&mut self, mut callback: C)
    where
        C: FnMut(usize) -> ServerMessage,
    {
        match self {
            ServerTransportLayer::Memory { clients, .. } => {
                for (i, sender) in clients.iter_mut().enumerate() {
                    match sender.send(callback(i)) {
                        Ok(_) => {}
                        Err(err) => Log::err(format!("Unable to send server message: {}", err)),
                    }
                }
            }
            ServerTransportLayer::Tcp { connections, .. } => {
                for (i, client_connection) in connections.iter_mut().enumerate() {
                    match client_connection.send_message(&callback(i)) {
                        Ok(_) => {}
                        Err(err) => Log::err(format!("Unable to send server message: {}", err)),
                    }
                }
            }
        }
    }

    fn broadcast_message_to_clients(&mut self, message: ServerMessage) {
        self.broadcast_message_to_clients_callback(|_| message.clone())
    }

    pub fn read_client_messages<C>(&mut self, mut callback: C) -> GameResult
    where
        C: FnMut(ClientMessage) -> GameResult,
    {
        match self {
            ServerTransportLayer::Memory { receiver, .. } => {
                while let Ok(msg) = receiver.try_recv() {
                    callback(msg)?
                }
            }
            ServerTransportLayer::Tcp { connections, .. } => {
                for connection in connections.iter_mut() {
                    while let Some(msg) = connection.pop_message::<ClientMessage>() {
                        callback(msg)?
                    }
                }
            }
        }

        Ok(())
    }

    fn connection_addresses(&self) -> Vec<String> {
        match self {
            ServerTransportLayer::Memory { clients, .. } => {
                clients.iter().map(|c| format!("{c:p}")).collect()
            }
            ServerTransportLayer::Tcp { connections, .. } => connections
                .iter()
                .map(|c| c.string_peer_address())
                .collect(),
        }
    }
}

pub struct Server {
    pub transport: ServerTransportLayer,
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

    fn with_transport(transport: ServerTransportLayer) -> Self {
        Self {
            transport,
            previous_node_states: Default::default(),
            previous_sound_states: Default::default(),
            add_bots: true,
        }
    }

    pub fn new_tcp<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Ok(Self::with_transport(ServerTransportLayer::tcp(addr)?))
    }

    pub fn new_memory() -> Self {
        Self::with_transport(ServerTransportLayer::memory())
    }

    pub fn broadcast_message_to_clients(&mut self, message: ServerMessage) {
        self.transport.broadcast_message_to_clients(message)
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
        self.transport.read_client_messages(|msg| {
            info!("Message From Client: {msg:?}");

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
                    Ok(())
                }
            }
        })
    }

    pub fn on_scene_loaded(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        let scene = &mut ctx.scenes[scene];
        let players_to_spawn = self.transport.num_connections();

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
                self.transport
                    .broadcast_message_to_clients_callback(|connection_num| {
                        ServerMessage::AddPlayers(vec![PlayerDescriptor {
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
                        }])
                    })
            }
        }

        if self.add_bots {
            let bot_prefab =
                block_on(ctx.resource_manager.request::<Model>("data/models/bot.rgs")).unwrap();

            for i in players_to_spawn..start_points.len() {
                let ids = bot_prefab.generate_ids();

                if let Some(position) = start_points.get(i) {
                    self.transport
                        .broadcast_message_to_clients(ServerMessage::AddPlayers(vec![
                            PlayerDescriptor {
                                instance: InstanceDescriptor {
                                    path: "data/models/bot.rgs".into(),
                                    position: *position,
                                    rotation: Default::default(),
                                    velocity: Default::default(),
                                    ids: ids.clone(),
                                },
                                kind: ActorKind::Bot,
                            },
                        ]))
                }
            }
        }
    }

    pub fn address(&self) -> Option<SocketAddr> {
        match self.transport {
            ServerTransportLayer::Memory { .. } => None,
            ServerTransportLayer::Tcp { ref listener, .. } => listener.local_address().ok(),
        }
    }

    pub fn connection_addresses(&self) -> Vec<String> {
        self.transport.connection_addresses()
    }

    pub fn num_connections(&self) -> usize {
        self.transport.num_connections()
    }

    pub fn accept_connections(&mut self) {
        if let ServerTransportLayer::Tcp {
            ref mut connections,
            ref listener,
        } = self.transport
        {
            connections.extend(listener.accept_connections())
        }
    }
}
