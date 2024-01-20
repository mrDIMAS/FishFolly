//! Game project.
use crate::{
    bot::Bot, camera::CameraController, cannon::Cannon, client::Client, jumper::Jumper, menu::Menu,
    obstacle::RotatorObstacle, player::Player, respawn::RespawnZone, server::Server,
    start::StartPoint, target::Target,
};
use fyrox::{
    core::{log::Log, pool::Handle},
    event::{ElementState, Event, WindowEvent},
    gui::message::UiMessage,
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    renderer::QualitySettings,
    scene::{node::Node, Scene},
};
use std::{collections::HashSet, path::Path};

pub mod actor;
pub mod bot;
pub mod camera;
pub mod cannon;
pub mod client;
pub mod jumper;
pub mod menu;
pub mod net;
pub mod obstacle;
pub mod player;
pub mod respawn;
pub mod server;
pub mod start;
pub mod target;
pub mod utils;

#[derive(Default)]
pub struct DebugSettings {
    pub show_paths: bool,
    pub show_physics: bool,
    pub disable_ragdoll: bool,
}

pub struct Game {
    menu: Menu,
    scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
    pub debug_settings: DebugSettings,
    server: Option<Server>,
    client: Option<Client>,
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
            .add::<Jumper>("Jumper");
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
    fn new(_override_scene: Option<&str>, mut context: PluginContext) -> Self {
        Log::info("Game started!");

        Self {
            menu: Menu::new(&mut context),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            scene: Default::default(),
            debug_settings: Default::default(),
            server: None,
            client: None,
        }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!");
    }

    fn update(&mut self, ctx: &mut PluginContext) {
        if let Some(server) = self.server.as_mut() {
            server.accept_connections();
            server.read_messages();
        }
        if let Some(client) = self.client.as_mut() {
            client.read_messages(ctx);
        }

        if let Some(scene) = ctx.scenes.try_get_mut(self.scene) {
            scene.drawing_context.clear_lines();

            if self.debug_settings.show_physics {
                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }

        self.menu.update(ctx, &self.server);
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: PluginContext) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            if let PhysicalKey::Code(key_code) = event.physical_key {
                if event.state == ElementState::Pressed {
                    match key_code {
                        KeyCode::F1 => {
                            self.debug_settings.show_physics = !self.debug_settings.show_physics
                        }
                        KeyCode::F2 => {
                            self.debug_settings.show_paths = !self.debug_settings.show_paths
                        }
                        KeyCode::F3 => {
                            self.debug_settings.disable_ragdoll =
                                !self.debug_settings.disable_ragdoll
                        }
                        KeyCode::Escape => {
                            self.menu
                                .switch_main_menu_visibility(&_context.user_interface);
                        }
                        _ => (),
                    }
                }
            }
        }
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

    fn on_ui_message(&mut self, context: &mut PluginContext, message: &UiMessage) {
        self.menu
            .handle_ui_message(context, message, &mut self.server, &mut self.client);
    }

    fn on_scene_begin_loading(&mut self, _path: &Path, context: &mut PluginContext) {
        if self.scene.is_some() {
            context.scenes.remove(self.scene);
        }
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        _context: &mut PluginContext,
    ) {
        self.scene = scene;
        self.menu
            .set_main_menu_visibility(&_context.user_interface, false);
    }
}
