use crate::player::InputController;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    fxhash::FxHashMap,
    scene::{base::SceneNodeId, node::Node},
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NodeState {
    pub node: SceneNodeId,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InstanceDescriptor {
    pub path: PathBuf,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub velocity: Vector3<f32>, // Rigid body only.
    pub ids: FxHashMap<Handle<Node>, SceneNodeId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerDescriptor {
    pub instance: InstanceDescriptor,
    pub is_remote: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct SoundState {
    pub node: SceneNodeId,
    pub is_playing: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTickMessage {
    pub nodes: Vec<NodeState>,
    pub sounds: Vec<SoundState>,
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
        player: SceneNodeId,
        input_state: InputController,
    },
}
