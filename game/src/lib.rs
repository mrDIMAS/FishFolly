//! Game project.
use crate::{
    bot::Bot, camera::CameraController, message::Message, obstacle::RotatorObstacle,
    player::Player, respawn::RespawnZone, start::StartPoint, target::Target,
};
use fyrox::{
    core::{
        color::Color,
        futures::executor::block_on,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    event::Event,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::{
        node::{Node, TypeUuidProvider},
        Scene, SceneLoader,
    },
    utils::log::Log,
};
use std::{
    collections::HashSet,
    sync::mpsc::{self, Receiver, Sender},
};

pub mod bot;
pub mod camera;
pub mod marker;
pub mod message;
pub mod obstacle;
pub mod player;
pub mod respawn;
pub mod start;
pub mod target;

pub struct Game {
    scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
    pub message_sender: Sender<Message>,
    pub message_receiver: Receiver<Message>,
}

impl TypeUuidProvider for Game {
    // Returns unique plugin id for serialization needs.
    fn type_uuid() -> Uuid {
        uuid!("cb358b1c-fc23-4c44-9e59-0a9671324196")
    }
}

impl Game {
    pub fn new() -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        Self {
            scene: Default::default(),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            message_sender,
            message_receiver,
        }
    }

    fn set_scene(&mut self, scene: Handle<Scene>, context: PluginContext) {
        self.scene = scene;

        if let Some(scene) = context.scenes.try_get_mut(self.scene) {
            scene.ambient_lighting_color = Color::opaque(200, 200, 200);
        }

        Log::info("Scene was set successfully!".to_owned());
    }
}

impl Plugin for Game {
    fn on_register(&mut self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors.add::<Game, Player, _>("Player");
        script_constructors.add::<Game, CameraController, _>("Camera Controller");
        script_constructors.add::<Game, Bot, _>("Bot");
        script_constructors.add::<Game, Target, _>("Target");
        script_constructors.add::<Game, RotatorObstacle, _>("Rotator Obstacle");
        script_constructors.add::<Game, StartPoint, _>("Start Point");
        script_constructors.add::<Game, RespawnZone, _>("Respawn Zone");
    }

    fn on_standalone_init(&mut self, context: PluginContext) {
        let scene = block_on(
            block_on(SceneLoader::from_file(
                "data/scene.rgs",
                context.serialization_context.clone(),
            ))
            .unwrap()
            .finish(context.resource_manager.clone()),
        );

        self.set_scene(context.scenes.add(scene), context);
    }

    fn on_enter_play_mode(&mut self, scene: Handle<Scene>, context: PluginContext) {
        self.set_scene(scene, context);
    }

    fn on_leave_play_mode(&mut self, context: PluginContext) {
        self.set_scene(Handle::NONE, context)
    }

    fn on_unload(&mut self, _context: &mut PluginContext) {}

    fn update(&mut self, _context: &mut PluginContext) {
        while let Ok(message) = self.message_receiver.try_recv() {
            match message {
                Message::UnregisterTarget(target) => {
                    assert!(self.targets.remove(&target));
                }
                Message::UnregisterActor(actor) => {
                    assert!(self.actors.remove(&actor));
                }
                Message::UnregisterStartPoint(start_point) => {
                    assert!(self.start_points.remove(&start_point));
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_os_event(&mut self, _event: &Event<()>, _context: PluginContext) {}
}

pub fn game_ref(plugin: &dyn Plugin) -> &Game {
    plugin.cast::<Game>().unwrap()
}

pub fn game_mut(plugin: &mut dyn Plugin) -> &mut Game {
    plugin.cast_mut::<Game>().unwrap()
}
