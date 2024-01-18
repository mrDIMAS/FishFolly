//! Game project.
use crate::{
    bot::Bot, camera::CameraController, cannon::Cannon, jumper::Jumper, menu::Menu,
    obstacle::RotatorObstacle, player::Player, ragdoll::link::BoneLink, ragdoll::Ragdoll,
    respawn::RespawnZone, start::StartPoint, target::Target,
};
use fyrox::renderer::QualitySettings;
use fyrox::{
    core::{log::Log, pool::Handle},
    event::Event,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{node::Node, Scene},
};
use std::collections::HashSet;

pub mod bot;
pub mod camera;
pub mod cannon;
pub mod jumper;
pub mod marker;
pub mod menu;
pub mod obstacle;
pub mod player;
pub mod ragdoll;
pub mod respawn;
pub mod start;
pub mod target;
pub mod utils;

pub struct Game {
    menu: Menu,
    scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
}

pub struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors
            .add::<Player>("Player")
            .add::<CameraController>("Camera Controller")
            .add::<Bot>("Bot")
            .add::<Target>("Target")
            .add::<RotatorObstacle>("Rotator Obstacle")
            .add::<StartPoint>("Start Point")
            .add::<RespawnZone>("Respawn Zone")
            .add::<Cannon>("Cannon")
            .add::<Jumper>("Jumper")
            .add::<Ragdoll>("Ragdoll")
            .add::<BoneLink>("Bone Link");
    }

    fn create_instance(
        &self,
        override_scene: Option<&str>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game::new(override_scene, context))
    }
}

impl Game {
    fn new(override_scene: Option<&str>, mut context: PluginContext) -> Self {
        Log::info("Game started!");

        context
            .async_scene_loader
            .request(override_scene.unwrap_or("data/drake.rgs"));

        Self {
            menu: Menu::new(&mut context),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            scene: Default::default(),
        }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!");
    }

    fn on_graphics_context_initialized(&mut self, context: PluginContext) {
        let graphics_context = context.graphics_context.as_initialized_mut();

        graphics_context.window.set_title("Fish Folly");

        let quality_settings = QualitySettings {
            use_ssao: false,
            ..Default::default()
        };

        Log::verify(
            context
                .graphics_context
                .as_initialized_mut()
                .renderer
                .set_quality_settings(&quality_settings),
        );
    }

    fn update(&mut self, context: &mut PluginContext) {
        if false {
            if let Some(scene) = context.scenes.try_get_mut(self.scene) {
                scene.drawing_context.clear_lines();

                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) {
        self.menu.handle_os_event(event, context);
    }

    fn on_ui_message(&mut self, context: &mut PluginContext, message: &UiMessage) {
        self.menu.handle_ui_message(context, message);
    }
}
