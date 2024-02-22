//! Game project.
use crate::{
    bot::Bot, camera::CameraController, cannon::Cannon, client::Client, jumper::Jumper,
    level::Level, menu::Menu, obstacle::RotatorObstacle, player::Player, respawn::Respawner,
    server::Server, settings::Settings, start::StartPoint, target::Target, trigger::Trigger,
};
use fyrox::{
    core::{log::Log, pool::Handle},
    event::{ElementState, Event, WindowEvent},
    gui::{message::UiMessage, UserInterface},
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::Scene,
};
use std::path::Path;

pub mod actor;
pub mod bot;
pub mod camera;
pub mod cannon;
pub mod client;
pub mod jumper;
pub mod level;
pub mod menu;
pub mod net;
pub mod obstacle;
pub mod player;
pub mod respawn;
pub mod server;
pub mod settings;
pub mod start;
pub mod target;
pub mod trigger;
pub mod utils;

#[derive(Default)]
pub struct DebugSettings {
    pub show_paths: bool,
    pub show_physics: bool,
    pub disable_ragdoll: bool,
}

pub struct Game {
    menu: Option<Menu>,
    pub level: Level,
    pub debug_settings: DebugSettings,
    server: Option<Server>,
    client: Option<Client>,
    settings: Settings,
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
            .add::<Respawner>("Respawner")
            .add::<Cannon>("Cannon")
            .add::<Trigger>("Trigger")
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
    fn new(_override_scene: Option<&str>, ctx: PluginContext) -> Self {
        Log::info("Game started!");

        ctx.task_pool.spawn_plugin_task(
            UserInterface::load_from_file("data/menu.ui", ctx.resource_manager.clone()),
            |result, game: &mut Game, ctx| match result {
                Ok(menu) => {
                    *ctx.user_interface = menu;
                    game.menu = Some(Menu::new(ctx, &game.settings));
                }
                Err(e) => Log::err(format!("Unable to load main menu! Reason: {:?}", e)),
            },
        );

        Self {
            menu: None,
            level: Default::default(),
            debug_settings: Default::default(),
            server: None,
            client: None,
            settings: Settings::load(),
        }
    }

    pub fn is_client(&self) -> bool {
        self.server.is_none() && self.client.is_some()
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!");
    }

    fn update(&mut self, ctx: &mut PluginContext) {
        if let Some(server) = self.server.as_mut() {
            server.accept_connections();

            server.read_messages(self.level.scene, ctx);
            server.update(self.level.scene, ctx);
        }

        if let Some(client) = self.client.as_mut() {
            client.read_messages(self.level.scene, ctx);
        }

        if let Some(scene) = ctx.scenes.try_get_mut(self.level.scene) {
            scene.drawing_context.clear_lines();

            if self.debug_settings.show_physics {
                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }

        if let Some(menu) = self.menu.as_mut() {
            menu.update(ctx, &self.server);
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, ctx: PluginContext) {
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
                            if let Some(menu) = self.menu.as_ref() {
                                menu.switch_visibility(ctx.user_interface, self.client.is_some());
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn on_graphics_context_initialized(&mut self, ctx: PluginContext) {
        self.settings
            .read()
            .apply_graphics_settings(ctx.graphics_context);
        ctx.graphics_context
            .as_initialized_mut()
            .window
            .set_title("Fish Folly");
    }

    fn on_ui_message(&mut self, context: &mut PluginContext, message: &UiMessage) {
        if let Some(menu) = self.menu.as_mut() {
            menu.handle_ui_message(
                context,
                message,
                &mut self.server,
                &mut self.client,
                &mut self.settings,
            );
        }
    }

    fn on_scene_begin_loading(&mut self, _path: &Path, context: &mut PluginContext) {
        if self.level.scene.is_some() {
            context.scenes.remove(self.level.scene);
        }
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        ctx: &mut PluginContext,
    ) {
        self.settings.read().apply_sound_volume(ctx, scene);

        self.level = Level {
            scene,
            ..Default::default()
        };

        if let Some(menu) = self.menu.as_ref() {
            menu.set_main_menu_visibility(ctx.user_interface, false);
        }
        if let Some(server) = self.server.as_mut() {
            server.on_scene_loaded(scene, ctx);
        }
        if let Some(client) = self.client.as_mut() {
            client.on_scene_loaded(self.server.is_some(), scene, ctx);
        }
    }
}
