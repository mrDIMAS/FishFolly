use crate::{
    net::{
        ClientMessage, NetListener, NetStream, NodeState, PlayerDescriptor, ServerMessage,
        UpdateTickMessage,
    },
    player::Player,
    start::StartPoint,
};
use fyrox::{
    core::{log::Log, pool::Handle},
    fxhash::FxHashMap,
    plugin::PluginContext,
    scene::{node::Node, rigidbody::RigidBody, Scene},
};
use std::io;

pub struct Server {
    listener: NetListener,
    connections: Vec<NetStream>,
    previous_node_states: FxHashMap<Handle<Node>, NodeState>,
}

impl Server {
    pub const ADDRESS: &'static str = "127.0.0.1:10001"; // TODO

    pub fn new() -> io::Result<Self> {
        Ok(Self {
            listener: NetListener::bind(Self::ADDRESS)?,
            connections: Default::default(),
            previous_node_states: Default::default(),
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

    pub fn start_game(&mut self) {
        self.broadcast_message_to_clients(ServerMessage::LoadLevel {
            path: "data/drake.rgs".into(),
        });
    }

    pub fn update(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        if let Some(scene) = ctx.scenes.try_get_mut(scene) {
            let mut tick_data = UpdateTickMessage {
                nodes: Default::default(),
            };

            for (handle, node) in scene.graph.pair_iter() {
                let current_state =
                    if let Some(rigid_body) = node.query_component_ref::<RigidBody>() {
                        NodeState {
                            node: handle,
                            position: **rigid_body.local_transform().position(),
                            rotation: **rigid_body.local_transform().rotation(),
                            velocity: rigid_body.lin_vel(),
                            angular_velocity: rigid_body.ang_vel(),
                        }
                    } else {
                        NodeState {
                            node: handle,
                            position: **node.local_transform().position(),
                            rotation: **node.local_transform().rotation(),
                            velocity: Default::default(),
                            angular_velocity: Default::default(),
                        }
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
            }

            self.broadcast_message_to_clients(ServerMessage::UpdateTick(tick_data));
        }
    }

    pub fn read_messages(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        for player in self.connections.iter_mut() {
            player.process_input::<ClientMessage>(|msg| match msg {
                ClientMessage::Input {
                    player,
                    input_state,
                } => {
                    let scene = &mut ctx.scenes[scene];
                    if let Some(player_ref) = scene
                        .graph
                        .try_get_script_component_of_mut::<Player>(player)
                    {
                        player_ref.input_controller = input_state;
                    }
                }
            });
        }
    }

    pub fn on_scene_loaded(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        let scene = &mut ctx.scenes[scene];
        let mut players_to_spawn = self.connections.len();

        let start_points = scene
            .graph
            .linear_iter()
            .filter(|n| n.has_script::<StartPoint>())
            .map(|n| n.global_position())
            .collect::<Vec<_>>();

        for player_num in 0..players_to_spawn {
            if let Some(position) = start_points.get(player_num) {
                for (connection_num, connection) in self.connections.iter_mut().enumerate() {
                    connection
                        .send_message(&ServerMessage::AddPlayers(vec![PlayerDescriptor {
                            path: "data/models/player.rgs".into(),
                            position: *position,
                            is_remote: player_num != connection_num,
                        }]))
                        .unwrap();
                }
            }
        }
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
