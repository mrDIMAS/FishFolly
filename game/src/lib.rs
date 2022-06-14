//! Game project.
use crate::{camera::CameraController, player::Player};
use fyrox::{
    core::{
        futures::executor::block_on,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    event::Event,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::{node::TypeUuidProvider, Scene, SceneLoader},
    utils::log::Log,
};

mod camera;
mod player;

pub struct Game {
    scene: Handle<Scene>,
}

impl TypeUuidProvider for Game {
    // Returns unique plugin id for serialization needs.
    fn type_uuid() -> Uuid {
        uuid!("cb358b1c-fc23-4c44-9e59-0a9671324196")
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            scene: Default::default(),
        }
    }

    fn set_scene(&mut self, scene: Handle<Scene>, _context: PluginContext) {
        self.scene = scene;

        Log::info("Scene was set successfully!".to_owned());
    }
}

impl Plugin for Game {
    fn on_register(&mut self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors.add::<Game, Player, _>("Player");
        script_constructors.add::<Game, CameraController, _>("Camera Controller");
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

    fn update(&mut self, _context: &mut PluginContext) {}

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_os_event(&mut self, _event: &Event<()>, _context: PluginContext) {}
}
