//! Game project.
use crate::{
    bot::Bot, camera::CameraController, obstacle::RotatorObstacle, player::Player,
    respawn::RespawnZone, start::StartPoint, target::Target,
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
use std::collections::HashSet;

pub mod bot;
pub mod camera;
pub mod marker;
pub mod obstacle;
pub mod player;
pub mod respawn;
pub mod start;
pub mod target;

#[derive(Default)]
pub struct Game {
    scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
}

impl TypeUuidProvider for Game {
    // Returns unique plugin id for serialization needs.
    fn type_uuid() -> Uuid {
        uuid!("cb358b1c-fc23-4c44-9e59-0a9671324196")
    }
}

impl Game {
    fn set_scene(&mut self, scene: Handle<Scene>, context: PluginContext) {
        self.scene = scene;

        if let Some(scene) = context.scenes.try_get_mut(self.scene) {
            scene.ambient_lighting_color = Color::opaque(200, 200, 200);

            Log::info("Scene was set successfully!".to_owned());
        }
    }
}

impl Plugin for Game {
    fn on_register(&mut self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors
            .add::<Game, Player, _>("Player")
            .add::<Game, CameraController, _>("Camera Controller")
            .add::<Game, Bot, _>("Bot")
            .add::<Game, Target, _>("Target")
            .add::<Game, RotatorObstacle, _>("Rotator Obstacle")
            .add::<Game, StartPoint, _>("Start Point")
            .add::<Game, RespawnZone, _>("Respawn Zone");
    }

    fn on_init(&mut self, override_scene: Handle<Scene>, context: PluginContext) {
        Log::info("Game started!".to_owned());

        let scene = if override_scene.is_some() {
            override_scene
        } else {
            let scene = block_on(
                block_on(SceneLoader::from_file(
                    "data/scene.rgs",
                    context.serialization_context.clone(),
                ))
                .unwrap()
                .finish(context.resource_manager.clone()),
            );

            context.scenes.add(scene)
        };

        self.set_scene(scene, context);
    }

    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!".to_owned());
    }

    fn update(&mut self, _context: &mut PluginContext) {}

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
