use crate::player::InputController;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    scene::node::Node,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeState {
    pub node: Handle<Node>,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub velocity: Vector3<f32>,         // Rigid body only.
    pub angular_velocity: Vector3<f32>, // Rigid body only.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceDescriptor {
    pub path: PathBuf,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub velocity: Vector3<f32>, // Rigid body only.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerDescriptor {
    pub path: PathBuf,
    pub position: Vector3<f32>,
    pub is_remote: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTickMessage {
    pub nodes: Vec<NodeState>,
}

/// A message sent from the server to a client.
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    LoadLevel { path: PathBuf },
    UpdateTick(UpdateTickMessage),
    AddPlayers(Vec<PlayerDescriptor>),
    Instantiate(Vec<InstanceDescriptor>),
}

/// A message sent from a client to the server.
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Input {
        player: Handle<Node>,
        input_state: InputController,
    },
}
