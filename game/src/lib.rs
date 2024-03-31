//! Game project.
use fyrox::{
    core::{log::Log, pool::Handle, visitor::prelude::*},
    event::{ElementState, Event, WindowEvent},
    gui::{
        inspector::editors::{
            inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
        },
        message::UiMessage,
        UserInterface,
    },
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::Scene,
    window::Fullscreen,
};
use std::path::Path;

use crate::{
    actor::Actor, bot::Bot, camera::CameraController, cannon::Cannon, client::Client,
    jumper::Jumper, level::Level, menu::Menu, player::Player, respawn::RespawnMode,
    respawn::Respawner, server::Server, settings::Settings, start::StartPoint, target::Target,
    trigger::Action, trigger::Trigger,
};
pub use fyrox;

pub mod actor;
pub mod bot;
pub mod camera;
pub mod cannon;
pub mod client;
pub mod jumper;
pub mod level;
pub mod menu;
pub mod net;
pub mod player;
pub mod respawn;
pub mod server;
pub mod settings;
pub mod start;
pub mod target;
pub mod trigger;
pub mod utils;

#[derive(Default, Visit)]
pub struct DebugSettings {
    pub show_paths: bool,
    pub show_physics: bool,
    pub disable_ragdoll: bool,
}

pub struct Game {
    pub menu: Option<Menu>,
    pub level: Level,
    pub debug_settings: DebugSettings,
    server: Option<Server>,
    client: Option<Client>,
    settings: Settings,
}

impl Visit for Game {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let _ = self.menu.visit("Menu", &mut region);
        let _ = self.level.visit("Level", &mut region);
        let _ = self.debug_settings.visit("DebugSettings", &mut region);

        if self.server.as_ref().map_or(0, |s| s.connections().len()) > 1 {
            Log::warn("Hot reloading is not possible when there's more than one client!");
        }

        let mut server_address = self.server.as_ref().map(|s| s.address().to_string());
        let _ = server_address.visit("ServerAddress", &mut region);

        if region.is_reading() {
            if let Some(address) = server_address {
                self.server = Some(Server::new(address.clone()).unwrap());
                self.client = Some(Client::try_connect(address).unwrap());
            }

            self.settings = Settings::load();
        }

        Ok(())
    }
}

impl Game {
    pub fn new() -> Self {
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
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors
            .add::<Player>("Player")
            .add::<CameraController>("Camera Controller")
            .add::<Bot>("Bot")
            .add::<Target>("Target")
            .add::<StartPoint>("Start Point")
            .add::<Respawner>("Respawner")
            .add::<Cannon>("Cannon")
            .add::<Trigger>("Trigger")
            .add::<Jumper>("Jumper");
    }

    fn register_property_editors(&self) -> PropertyEditorDefinitionContainer {
        let container = PropertyEditorDefinitionContainer::empty();
        container.insert(InspectablePropertyEditorDefinition::<Actor>::new());
        container.register_inheritable_enum::<RespawnMode, _>();
        container.register_inheritable_enum::<Action, _>();
        container
    }

    fn init(&mut self, _scene_path: Option<&str>, ctx: PluginContext) {
        Log::info("Game started!");

        ctx.task_pool.spawn_plugin_task(
            UserInterface::load_from_file("data/menu.ui", ctx.resource_manager.clone()),
            |result, game: &mut Game, ctx| match result {
                Ok(menu) => {
                    *ctx.user_interfaces.first_mut() = menu;
                    let menu = Some(Menu::new(ctx, game));
                    game.menu = menu;
                }
                Err(e) => Log::err(format!("Unable to load main menu! Reason: {:?}", e)),
            },
        );
    }

    fn on_deinit(&mut self, _context: PluginContext) {
        Log::info("Game stopped!");
    }

    fn update(&mut self, ctx: &mut PluginContext) {
        if let Some(server) = self.server.as_mut() {
            server.accept_connections();

            server.read_messages(self.level.scene, ctx);
            server.update(&mut self.level, ctx);
        }

        if let Some(client) = self.client.as_mut() {
            client.read_messages(&mut self.level, self.menu.as_ref(), ctx);
            client.update(ctx.dt);
        }

        if let Some(scene) = ctx.scenes.try_get_mut(self.level.scene) {
            scene.drawing_context.clear_lines();

            if self.debug_settings.show_physics {
                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }

        if let Some(menu) = self.menu.as_mut() {
            menu.update(ctx, &self.server, &self.client, &mut self.level);
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
                                menu.switch_visibility(
                                    ctx.user_interfaces.first(),
                                    self.client.is_some(),
                                );
                            }
                        }
                        KeyCode::F4 => {
                            self.level.match_timer = 3.0;
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

        if false {
            ctx.graphics_context
                .as_initialized_mut()
                .window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        }
    }

    fn on_ui_message(&mut self, context: &mut PluginContext, message: &UiMessage) {
        if let Some(menu) = self.menu.as_mut() {
            menu.handle_ui_message(
                context,
                message,
                &mut self.server,
                &mut self.client,
                &mut self.settings,
                self.level.scene,
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
        self.settings.read().apply_sound_volume(&ctx.scenes[scene]);

        self.level = Level {
            scene,
            ..Default::default()
        };

        if let Some(menu) = self.menu.as_ref() {
            self.level.leaderboard.sender = Some(menu.sender.clone());
            menu.set_menu_visibility(ctx.user_interfaces.first(), false);
        }
        if let Some(server) = self.server.as_mut() {
            server.on_scene_loaded(scene, ctx);
        }
        if let Some(client) = self.client.as_mut() {
            client.on_scene_loaded(self.server.is_some(), scene, ctx);
        }
    }
}
