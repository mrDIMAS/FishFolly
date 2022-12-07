//! Game project.
use crate::{
    bot::Bot, camera::CameraController, cannon::Cannon, jumper::Jumper, menu::Menu,
    obstacle::RotatorObstacle, player::Player, ragdoll::link::BoneLink, ragdoll::Ragdoll,
    respawn::RespawnZone, start::StartPoint, target::Target,
};
use fyrox::{
    core::pool::Handle,
    event::Event,
    event_loop::ControlFlow,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{loader::AsyncSceneLoader, node::Node, Scene},
    utils::log::Log,
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
    loader: Option<AsyncSceneLoader>,
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
        override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game::new(override_scene, context))
    }
}

impl Game {
    fn new(override_scene: Handle<Scene>, mut context: PluginContext) -> Self {
        Log::info("Game started!");

        let mut loader = None;
        let scene = if override_scene.is_some() {
            override_scene
        } else {
            loader = Some(AsyncSceneLoader::begin_loading(
                "data/drake.rgs".into(),
                context.serialization_context.clone(),
                context.resource_manager.clone(),
            ));
            Default::default()
        };

        Self {
            menu: Menu::new(&mut context),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            scene,
            loader,
        }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!");
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        self.menu.handle_os_event(event, context);
    }

    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        if let Some(loader) = self.loader.as_ref() {
            if let Some(result) = loader.fetch_result() {
                match result {
                    Ok(scene) => {
                        self.scene = context.scenes.add(scene);
                    }
                    Err(err) => Log::err(err),
                }
            }
        }

        if false {
            if let Some(scene) = context.scenes.try_get_mut(self.scene) {
                scene.drawing_context.clear_lines();

                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }
    }

    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &UiMessage,
        control_flow: &mut ControlFlow,
    ) {
        self.menu.handle_ui_message(context, message, control_flow);
    }
}

pub fn game_ref(plugins: &[Box<dyn Plugin>]) -> &Game {
    plugins.first().unwrap().cast::<Game>().unwrap()
}

pub fn game_mut(plugins: &mut [Box<dyn Plugin>]) -> &mut Game {
    plugins.first_mut().unwrap().cast_mut::<Game>().unwrap()
}
