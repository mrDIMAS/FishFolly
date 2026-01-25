use crate::{
    actor::{Actor, ActorKind},
    client::Client,
    level::{LeaderBoardEvent, Level},
    server::Server,
    settings::Settings,
    utils, Game,
};
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        log::Log, pool::Handle, pool::HandlesVecExtension, reflect::prelude::*,
        type_traits::prelude::*, visitor::prelude::*,
    },
    engine::GraphicsContext,
    graph::SceneGraph,
    gui::{
        animation::{AnimationPlayer, AnimationPlayerMessage},
        button::{Button, ButtonMessage},
        check_box::{CheckBox, CheckBoxMessage},
        font::Font,
        list_view::{ListView, ListViewMessage},
        message::UiMessage,
        scroll_bar::{ScrollBar, ScrollBarMessage},
        selector::{Selector, SelectorMessage},
        text::{Text, TextBuilder, TextMessage},
        text_box::TextBox,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    plugin::{error::GameResult, PluginContext},
    resource::model::Model,
    scene::{graph::Graph, node::Node, sound::Sound, Scene, SceneContainer},
};
use std::{
    ffi::OsStr,
    fmt::Debug,
    net::ToSocketAddrs,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

pub fn make_text_widget(
    ctx: &mut BuildContext,
    name: &str,
    resource_manager: &ResourceManager,
    horizontal_alignment: HorizontalAlignment,
) -> Handle<Text> {
    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(horizontal_alignment)
        .with_text(name)
        .with_font(resource_manager.request::<Font>("data/font.ttf"))
        .with_font_size(28.0.into())
        .build(ctx)
}

fn set_visibility(ui: &UserInterface, pairs: &[(Handle<UiNode>, bool)]) {
    for (widget, visibility) in pairs {
        ui.send(*widget, WidgetMessage::Visibility(*visibility));
    }
}

#[derive(Visit, Reflect, Debug, Default, Clone, TypeUuidProvider)]
#[type_uuid(id = "7dc2d3b9-1990-464c-bab3-3b6973f930e9")]
pub struct ServerMenu {
    self_handle: Handle<UiNode>,
    main_menu: Handle<UiNode>,
    back: Handle<Button>,
    players_list: Handle<ListView>,
    start: Handle<Button>,
    server_address_input: Handle<TextBox>,
    add_bots_check_box: Handle<CheckBox>,
    level_selector: Handle<Selector>,
    #[reflect(hidden)]
    server_address: String,
    #[reflect(hidden)]
    available_levels: Vec<PathBuf>,
    #[reflect(hidden)]
    selected_level: Option<usize>,
}

impl ServerMenu {
    pub fn fill_levels_list(&mut self, ui: &mut UserInterface, resource_manager: &ResourceManager) {
        self.available_levels = walkdir::WalkDir::new("./data/maps")
            .into_iter()
            .filter_map(|result| result.ok())
            .filter(|entry| entry.path().extension() == Some(OsStr::new("rgs")))
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<_>>();

        let levels_list_items = self
            .available_levels
            .iter()
            .map(|path| {
                make_text_widget(
                    &mut ui.build_ctx(),
                    &path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    resource_manager,
                    HorizontalAlignment::Center,
                )
            })
            .collect::<Vec<_>>();

        if !levels_list_items.is_empty() {
            ui.send(self.level_selector, SelectorMessage::Current(Some(0)))
        }
        ui.send(
            self.level_selector,
            SelectorMessage::SetItems {
                items: levels_list_items.to_base(),
                remove_previous: true,
            },
        );

        self.server_address = "127.0.0.1:10001".to_string();
        self.selected_level = self.available_levels.first().map(|_| 0);
    }

    pub fn handle_ui_message(
        &mut self,
        ctx: &mut PluginContext,
        message: &UiMessage,
        server: &mut Option<Server>,
    ) {
        let ui = ctx.user_interfaces.first();

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.start {
                ui.send(self.self_handle, WidgetMessage::Visibility(false));
                if let Some(selected_level) = self.selected_level {
                    if let Some(server) = server.as_mut() {
                        server.start_game(&self.available_levels[selected_level]);
                    }
                }
            } else if message.destination() == self.back {
                ui.send(self.self_handle, WidgetMessage::Visibility(false));
                ui.send(self.main_menu, WidgetMessage::Visibility(true));
                *server = None;
            }
        } else if let Some(TextMessage::Text(text)) = message.data_from(self.server_address_input) {
            self.server_address = text.clone();
        } else if let Some(SelectorMessage::Current(selected)) =
            message.data_from(self.level_selector)
        {
            self.selected_level = *selected;
        } else if let Some(CheckBoxMessage::Check(Some(value))) =
            message.data_from(self.add_bots_check_box)
        {
            if let Some(server) = server {
                server.add_bots = *value;
            }
        }
    }

    pub fn update(&self, ctx: &mut PluginContext, server: &Option<Server>) {
        let Some(server) = server else {
            return;
        };

        let player_entries_count = ctx.user_interfaces.first()[self.players_list].items().len();
        if server.connections().len() != player_entries_count {
            let new_player_entries = server
                .connections()
                .iter()
                .enumerate()
                .map(|(n, e)| {
                    make_text_widget(
                        &mut ctx.user_interfaces.first_mut().build_ctx(),
                        &format!(
                            "{} - {}",
                            e.string_peer_address(),
                            if n == 0 { "Host" } else { "Peer" }
                        ),
                        ctx.resource_manager,
                        HorizontalAlignment::Left,
                    )
                })
                .collect::<Vec<_>>();
            ctx.user_interfaces.first().send(
                self.players_list,
                ListViewMessage::Items(new_player_entries.to_base()),
            );
        }
    }
}

#[derive(Visit, Reflect, Debug, Default, Clone, TypeUuidProvider)]
#[type_uuid(id = "556115c2-6f30-4bca-98cf-b94a0810f38c")]
pub struct SettingsMenu {
    menu: Handle<UiNode>,
    graphics_quality: Handle<Selector>,
    sound_volume: Handle<ScrollBar>,
    music_volume: Handle<ScrollBar>,
    mouse_sens: Handle<ScrollBar>,
    mouse_smoothness: Handle<ScrollBar>,
    back: Handle<Button>,
    reset: Handle<Button>,
}

impl SettingsMenu {
    pub fn sync_with_settings(
        &mut self,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        settings: &Settings,
    ) {
        let settings = settings.read();

        let items = settings
            .graphics_presets
            .iter()
            .map(|(name, _)| {
                make_text_widget(
                    &mut ui.build_ctx(),
                    name,
                    resource_manager,
                    HorizontalAlignment::Center,
                )
            })
            .collect::<Vec<_>>();

        ui.send(
            self.graphics_quality,
            SelectorMessage::SetItems {
                items: items.to_base(),
                remove_previous: true,
            },
        );
        ui.send(
            self.graphics_quality,
            SelectorMessage::Current(Some(settings.graphics_quality)),
        );

        fn set_sb_value(ui: &UserInterface, handle: Handle<ScrollBar>, value: f32) {
            ui.send(handle, ScrollBarMessage::Value(value));
        }
        set_sb_value(ui, self.sound_volume, settings.sound_volume);
        set_sb_value(ui, self.music_volume, settings.music_volume);
        set_sb_value(ui, self.mouse_sens, settings.mouse_sensitivity);
        set_sb_value(ui, self.mouse_smoothness, settings.mouse_smoothness);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        main_menu: Handle<UiNode>,
        ui: &UserInterface,
        graphics_context: &mut GraphicsContext,
        settings: &mut Settings,
        scenes: &SceneContainer,
        game_scene: Handle<Scene>,
        menu_scene: Handle<Scene>,
    ) -> GameResult {
        if let Some(SelectorMessage::Current(Some(index))) = message.data() {
            if message.destination() == self.graphics_quality {
                let mut settings = settings.write();
                settings.graphics_quality = *index;
                settings.apply_graphics_settings(graphics_context);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.back {
                set_visibility(ui, &[(self.menu, false), (main_menu, true)]);
            } else if message.destination() == self.reset {
            }
        } else if let Some(ScrollBarMessage::Value(value)) = message.data() {
            if message.destination() == self.sound_volume {
                let mut settings = settings.write();
                settings.sound_volume = *value;
                if let Ok(game_scene) = scenes.try_get(game_scene) {
                    settings.apply_sound_volume(game_scene);
                }
            } else if message.destination() == self.music_volume {
                let mut settings = settings.write();
                settings.apply_music_volume(scenes.try_get(menu_scene)?);
                settings.music_volume = *value;
            } else if message.destination() == self.mouse_sens {
                settings.write().mouse_sensitivity = *value;
            } else if message.destination() == self.mouse_smoothness {
                settings.write().mouse_smoothness = *value;
            }
        }
        Ok(())
    }
}

#[derive(Visit, Reflect, Debug, Default, Clone, TypeUuidProvider)]
#[type_uuid(id = "24d6e2ad-918c-45db-987b-3605d70469c2")]
pub struct InGameMenu {
    root: Handle<UiNode>,
    finished_text: Handle<Text>,
    finished_text_animation: Handle<AnimationPlayer>,
    match_timer_text: Handle<Text>,
    player_position: Handle<Text>,
}

impl InGameMenu {
    fn on_leaderboard_event(
        &self,
        ui: &mut UserInterface,
        game_scene: &Scene,
        event: &LeaderBoardEvent,
    ) -> GameResult {
        match event {
            LeaderBoardEvent::Finished { actor, place } => {
                let actor = game_scene
                    .graph
                    .try_get_script_component_of::<Actor>(*actor)?;

                let suffix = match place {
                    1 => "st",
                    2 => "nd",
                    3 => "d",
                    _ => "th",
                };
                ui.send(
                    self.finished_text,
                    TextMessage::Text(format!("{} finished {place}{suffix}", actor.name)),
                );

                fn enable_animation(
                    ui: &UserInterface,
                    widget: Handle<AnimationPlayer>,
                    name: &str,
                ) {
                    ui.send(
                        widget,
                        AnimationPlayerMessage::EnableAnimation {
                            animation: name.to_string(),
                            enabled: true,
                        },
                    );
                }
                let id = "Animation".to_string();
                enable_animation(ui, self.finished_text_animation, "Animation");
                enable_animation(ui, self.finished_text_animation, &id);
                ui.send(
                    self.finished_text_animation,
                    AnimationPlayerMessage::RewindAnimation { animation: id },
                );
            }
        }
        Ok(())
    }

    fn update(&self, ui: &UserInterface, graph: Option<&Graph>, level: &Level) -> GameResult {
        let minutes = (level.match_timer / 60.0) as u32;
        let seconds = (level.match_timer % 60.0) as u32;
        ui.send(
            self.match_timer_text,
            TextMessage::Text(format!("{minutes}:{seconds}")),
        );
        ui.send(self.root, WidgetMessage::Visibility(level.scene.is_some()));

        if let Some(graph) = graph {
            for (actor, entry) in &level.leaderboard.entries {
                let actor_ref = graph.try_get_script_component_of::<Actor>(*actor)?;
                if actor_ref.kind == ActorKind::Player {
                    ui.send(
                        self.player_position,
                        TextMessage::Text(format!(
                            "Place: {} of {}",
                            entry.real_time_position + 1,
                            level.actors.len()
                        )),
                    );

                    break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Visit, Reflect, Debug, TypeUuidProvider)]
#[type_uuid(id = "87b01b49-af2b-439a-a077-61700f817e3e")]
pub struct LeaderBoardChannel {
    #[visit(skip)]
    #[reflect(hidden)]
    pub sender: Sender<LeaderBoardEvent>,
    #[visit(skip)]
    #[reflect(hidden)]
    receiver: Receiver<LeaderBoardEvent>,
}

impl Default for LeaderBoardChannel {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }
}

impl Clone for LeaderBoardChannel {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider)]
#[type_uuid(id = "82708d44-2abe-4792-b144-ef70c26cb693")]
pub struct MenuSceneData {
    click_begin_sound: Handle<Sound>,
    click_end_sound: Handle<Sound>,
    root_scene_node: Handle<Node>,
    finished_sound: Handle<Sound>,
    win_camera: Handle<Node>,
    main_camera: Handle<Node>,
    clock_ticking: Handle<Sound>,
}

#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider)]
#[type_uuid(id = "87b01b49-af2b-439a-a077-61700f817e3e")]
pub struct MenuData {
    server_menu: ServerMenu,
    settings_menu: SettingsMenu,
    in_game_menu: InGameMenu,
    debug_text: Handle<Text>,
    settings: Handle<Button>,
    exit: Handle<Button>,
    start_as_server: Handle<Button>,
    start_as_client: Handle<Button>,
    main_menu: Handle<UiNode>,
    main_menu_root: Handle<UiNode>,
    background: Handle<UiNode>,
}

#[derive(Debug, Clone, Reflect, Visit, Default)]
pub struct Menu {
    scene: Handle<Scene>,
    pub menu_data: MenuData,
    pub menu_scene_data: MenuSceneData,
    pub leader_board_channel: LeaderBoardChannel,
}

fn try_connect_to_server<A>(server_addr: A) -> Option<Client>
where
    A: ToSocketAddrs + Debug,
{
    match Client::try_connect(server_addr) {
        Ok(new_client) => Some(new_client),
        Err(err) => {
            Log::err(format!("Unable to create a client. Reason: {:?}", err));
            None
        }
    }
}

impl Menu {
    pub fn new(ctx: &mut PluginContext, game: &mut Game) -> Self {
        let settings = &game.settings;

        let ui = ctx.user_interfaces.first_mut();
        let mut menu_data = ui.user_data.try_take::<MenuData>().unwrap();
        menu_data
            .server_menu
            .fill_levels_list(ui, ctx.resource_manager);
        menu_data
            .settings_menu
            .sync_with_settings(ui, ctx.resource_manager, settings);

        ctx.task_pool.spawn_plugin_task(
            ctx.resource_manager
                .request::<Model>("data/models/menu.rgs"),
            |result, game: &mut Game, ctx| {
                let mut scene = result?.data_ref().get_scene().clone_one_to_one().0;
                let this = game.menu.as_mut().unwrap();
                this.menu_scene_data = scene.graph.user_data.try_take::<MenuSceneData>()?;
                this.scene = ctx.scenes.add(scene);
                Ok(())
            },
        );

        Self {
            scene: Default::default(),
            menu_data,
            menu_scene_data: Default::default(),
            leader_board_channel: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        ctx: &mut PluginContext,
        message: &UiMessage,
        server: &mut Option<Server>,
        client: &mut Option<Client>,
        settings: &mut Settings,
        game_scene: Handle<Scene>,
    ) -> GameResult {
        self.menu_data
            .server_menu
            .handle_ui_message(ctx, message, server);

        let ui = ctx.user_interfaces.first();

        self.menu_data.settings_menu.handle_ui_message(
            message,
            self.menu_data.main_menu,
            ui,
            ctx.graphics_context,
            settings,
            ctx.scenes,
            game_scene,
            self.scene,
        )?;

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.menu_data.exit {
                ctx.loop_controller.exit();
            } else if message.destination() == self.menu_data.start_as_server {
                set_visibility(
                    ui,
                    &[
                        (self.menu_data.server_menu.self_handle, true),
                        (self.menu_data.main_menu, false),
                    ],
                );
                ui.send(
                    self.menu_data.server_menu.server_address_input,
                    TextMessage::Text(Server::LOCALHOST.to_string()),
                );

                // Try to start the server and the client.
                match Server::new(&self.menu_data.server_menu.server_address) {
                    Ok(new_server) => {
                        *server = Some(new_server);
                        *client = try_connect_to_server(&self.menu_data.server_menu.server_address);
                        let server = server.as_mut().unwrap();
                        server.accept_connections();
                    }
                    Err(err) => Log::err(format!("Unable to create a server. Reason: {:?}", err)),
                }
            } else if message.destination() == self.menu_data.start_as_client {
                *client = try_connect_to_server(&self.menu_data.server_menu.server_address);
            } else if message.destination() == self.menu_data.settings {
                set_visibility(
                    ui,
                    &[
                        (self.menu_data.settings_menu.menu, true),
                        (self.menu_data.main_menu, false),
                    ],
                );
            }
        }

        if let Ok(scene) = ctx.scenes.try_get_mut(self.scene) {
            let graph = &mut scene.graph;
            if let Some(WidgetMessage::MouseDown { .. }) = message.data() {
                utils::try_play_sound(self.menu_scene_data.click_begin_sound, graph)?;
            } else if let Some(WidgetMessage::MouseUp { .. }) = message.data() {
                utils::try_play_sound(self.menu_scene_data.click_end_sound, graph)?;
            }
        }

        Ok(())
    }

    pub fn set_menu_visibility(&self, ui: &UserInterface, visible: bool) {
        ui.send(
            self.menu_data.main_menu_root,
            WidgetMessage::Visibility(visible),
        );
    }

    pub fn set_main_menu_visibility(&self, ui: &UserInterface, visible: bool) {
        ui.send(self.menu_data.main_menu, WidgetMessage::Visibility(visible));
    }

    pub fn switch_visibility(&self, ui: &UserInterface, is_client_running: bool) {
        let is_visible = ui.node(self.menu_data.main_menu_root).is_globally_visible();
        set_visibility(
            ui,
            &[
                (self.menu_data.main_menu_root, !is_visible),
                (self.menu_data.main_menu, !is_visible),
                (self.menu_data.server_menu.self_handle, false),
                (self.menu_data.background, !is_client_running),
            ],
        );
    }

    pub fn is_active(&self, ui: &UserInterface) -> bool {
        ui.try_get(self.menu_data.main_menu_root)
            .map(|n| n.is_globally_visible())
            .unwrap_or_default()
    }

    pub fn update(
        &self,
        ctx: &mut PluginContext,
        server: &Option<Server>,
        client: &Option<Client>,
        level: &mut Level,
    ) -> GameResult {
        let menu = &self.menu_data;
        let menu_scene = &self.menu_scene_data;

        menu.server_menu.update(ctx, server);

        if let GraphicsContext::Initialized(graphics_context) = ctx.graphics_context {
            let fps = graphics_context.renderer.get_statistics().frames_per_second;
            ctx.user_interfaces
                .first()
                .send(menu.debug_text, TextMessage::Text(format!("FPS: {fps}")));
        }

        if let Ok(scene) = ctx.scenes.try_get_mut(self.scene) {
            scene.graph[self.menu_scene_data.root_scene_node].set_visibility(level.scene.is_none());

            let mut is_in_win_state = false;
            if let Some(client) = client {
                is_in_win_state = client.win_context.is_some();
            }

            scene.graph[menu_scene.win_camera].set_enabled(is_in_win_state);
            scene.graph[menu_scene.main_camera].set_enabled(!is_in_win_state);
        }

        menu.in_game_menu.update(
            ctx.user_interfaces.first(),
            ctx.scenes.try_get_mut(level.scene).ok().map(|s| &s.graph),
            level,
        )?;

        while let Ok(event) = self.leader_board_channel.receiver.try_recv() {
            let game_scene = ctx.scenes.try_get_mut(level.scene)?;
            menu.in_game_menu.on_leaderboard_event(
                ctx.user_interfaces.first_mut(),
                game_scene,
                &event,
            )?;

            match event {
                LeaderBoardEvent::Finished { .. } => {
                    level.sudden_death();

                    let scene = ctx.scenes.try_get_mut(self.scene)?;
                    utils::try_play_sound(menu_scene.finished_sound, &mut scene.graph)?;
                    if level.is_time_critical() {
                        utils::try_play_sound(menu_scene.clock_ticking, &mut scene.graph)?;
                    }
                }
            }
        }

        Ok(())
    }
}
