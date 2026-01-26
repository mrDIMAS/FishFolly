//! Game project.
use crate::{
    actor::Actor,
    bot::Bot,
    camera::CameraController,
    cannon::Cannon,
    client::Client,
    jumper::Jumper,
    level::Level,
    menu::{InGameMenu, Menu, MenuData, MenuSceneData, ServerMenu, SettingsMenu},
    player::Player,
    respawn::{RespawnMode, Respawner},
    server::Server,
    settings::Settings,
    start::StartPoint,
    target::Target,
    trigger::{Action, Trigger},
};
pub use fyrox;
use fyrox::plugin::error;
use fyrox::{
    core::{log::Log, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    event::{ElementState, Event, WindowEvent},
    gui::{
        inspector::editors::{
            inspectable::InspectablePropertyEditorDefinition, PropertyEditorDefinitionContainer,
        },
        message::UiMessage,
        UserInterface,
    },
    keyboard::{KeyCode, PhysicalKey},
    plugin::{error::GameResult, Plugin, PluginContext, PluginRegistrationContext},
    scene::Scene,
    window::Fullscreen,
};
use std::{path::Path, sync::Arc};

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

#[derive(Default, Visit, Debug)]
pub struct DebugSettings {
    pub show_paths: bool,
    pub show_physics: bool,
    pub disable_ragdoll: bool,
}

#[derive(Reflect, Debug)]
#[reflect(hide_all, non_cloneable)]
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

impl Default for Game {
    fn default() -> Self {
        Self::new()
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
    fn register(&self, context: PluginRegistrationContext) -> GameResult {
        context
            .dyn_type_constructors
            .add::<MenuData>("Menu Data")
            .add::<MenuSceneData>("Menu Scene Data");
        context
            .serialization_context
            .script_constructors
            .add::<Player>("Player")
            .add::<CameraController>("Camera Controller")
            .add::<Bot>("Bot")
            .add::<Target>("Target")
            .add::<StartPoint>("Start Point")
            .add::<Respawner>("Respawner")
            .add::<Cannon>("Cannon")
            .add::<Trigger>("Trigger")
            .add::<Jumper>("Jumper");
        Ok(())
    }

    fn register_property_editors(&self, container: Arc<PropertyEditorDefinitionContainer>) {
        container.insert(InspectablePropertyEditorDefinition::<Actor>::new());
        container.insert(InspectablePropertyEditorDefinition::<Menu>::new());
        container.insert(InspectablePropertyEditorDefinition::<InGameMenu>::new());
        container.insert(InspectablePropertyEditorDefinition::<ServerMenu>::new());
        container.insert(InspectablePropertyEditorDefinition::<SettingsMenu>::new());
        container.register_inheritable_enum::<RespawnMode, _>();
        container.register_inheritable_enum::<Action, _>();
    }

    fn init(&mut self, _scene_path: Option<&str>, mut ctx: PluginContext) -> GameResult {
        Log::info("Game started!");

        error::enable_backtrace_capture(true);

        ctx.load_ui("data/menu.ui", |result, game: &mut Game, ctx| {
            game.menu = Some(Menu::new(result?, ctx, game));
            Ok(())
        });

        Ok(())
    }

    fn on_deinit(&mut self, _context: PluginContext) -> GameResult {
        Log::info("Game stopped!");
        Ok(())
    }

    fn update(&mut self, ctx: &mut PluginContext) -> GameResult {
        if let Some(server) = self.server.as_mut() {
            server.accept_connections();

            server.read_messages(self.level.scene, ctx)?;
            server.update(&mut self.level, ctx)?;
        }

        if let Some(client) = self.client.as_mut() {
            client.read_messages(&mut self.level, self.menu.as_ref(), ctx)?;
            client.update(ctx.dt);
        }

        if let Ok(scene) = ctx.scenes.try_get_mut(self.level.scene) {
            scene.drawing_context.clear_lines();

            if self.debug_settings.show_physics {
                scene.graph.physics.draw(&mut scene.drawing_context);
            }
        }

        if let Some(menu) = self.menu.as_mut() {
            menu.update(ctx, &self.server, &self.client, &mut self.level)?;
        }

        Ok(())
    }

    fn on_os_event(&mut self, event: &Event<()>, ctx: PluginContext) -> GameResult {
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

        Ok(())
    }

    fn on_graphics_context_initialized(&mut self, ctx: PluginContext) -> GameResult {
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

        Ok(())
    }

    fn on_ui_message(
        &mut self,
        context: &mut PluginContext,
        message: &UiMessage,
        _ui_handle: Handle<UserInterface>,
    ) -> GameResult {
        if let Some(menu) = self.menu.as_mut() {
            menu.handle_ui_message(
                context,
                message,
                &mut self.server,
                &mut self.client,
                &mut self.settings,
                self.level.scene,
            )?;
        }

        Ok(())
    }

    fn on_scene_begin_loading(&mut self, _path: &Path, context: &mut PluginContext) -> GameResult {
        if self.level.scene.is_some() {
            context.scenes.remove(self.level.scene);
        }

        Ok(())
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        ctx: &mut PluginContext,
    ) -> GameResult {
        self.settings.read().apply_sound_volume(&ctx.scenes[scene]);

        self.level = Level {
            scene,
            ..Default::default()
        };

        if let Some(menu) = self.menu.as_ref() {
            self.level.leaderboard.sender = Some(menu.leader_board_channel.sender.clone());
            menu.set_menu_visibility(ctx.user_interfaces.first(), false);
        }
        if let Some(server) = self.server.as_mut() {
            server.on_scene_loaded(scene, ctx);
        }
        if let Some(client) = self.client.as_mut() {
            client.on_scene_loaded(self.server.is_some(), scene, ctx)?;
        }

        Ok(())
    }
}
