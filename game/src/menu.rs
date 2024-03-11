use crate::{client::Client, server::Server, settings::Settings, utils};
use fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle},
    engine::GraphicsContext,
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        button::ButtonMessage,
        check_box::CheckBoxMessage,
        font::Font,
        list_view::{ListView, ListViewMessage},
        message::{MessageDirection, UiMessage},
        scroll_bar::ScrollBarMessage,
        selector::SelectorMessage,
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    plugin::PluginContext,
    scene::{
        base::BaseBuilder,
        node::Node,
        sound::{SoundBuffer, SoundBuilder},
        Scene, SceneContainer,
    },
};
use std::{ffi::OsStr, fmt::Debug, net::ToSocketAddrs, path::PathBuf};

pub fn make_text_widget(
    ctx: &mut BuildContext,
    name: &str,
    resource_manager: &ResourceManager,
    horizontal_alignment: HorizontalAlignment,
) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(horizontal_alignment)
        .with_text(name)
        .with_font(resource_manager.request::<Font>("data/font.ttf"))
        .with_font_size(28.0)
        .build(ctx)
}

fn set_visibility(ui: &UserInterface, pairs: &[(Handle<UiNode>, bool)]) {
    for (widget, visibility) in pairs {
        ui.send_message(WidgetMessage::visibility(
            *widget,
            MessageDirection::ToWidget,
            *visibility,
        ));
    }
}

#[derive(Default)]
struct ServerMenu {
    self_handle: Handle<UiNode>,
    main_menu: Handle<UiNode>,
    back: Handle<UiNode>,
    players_list: Handle<UiNode>,
    start: Handle<UiNode>,
    server_address_input: Handle<UiNode>,
    add_bots_check_box: Handle<UiNode>,
    server_address: String,
    level_selector: Handle<UiNode>,
    available_levels: Vec<PathBuf>,
    selected_level: Option<usize>,
}

impl ServerMenu {
    pub fn new(
        self_handle: Handle<UiNode>,
        main_menu: Handle<UiNode>,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) -> Self {
        let level_selector = ui.find_handle_by_name_from_root("SVLevelSelector");

        let available_levels = walkdir::WalkDir::new("./data/maps")
            .into_iter()
            .filter_map(|result| result.ok())
            .filter(|entry| entry.path().extension() == Some(OsStr::new("rgs")))
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<_>>();

        let levels_list_items = available_levels
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
            ui.send_message(SelectorMessage::current(
                level_selector,
                MessageDirection::ToWidget,
                Some(0),
            ))
        }
        ui.send_message(SelectorMessage::set_items(
            level_selector,
            MessageDirection::ToWidget,
            levels_list_items,
            true,
        ));

        Self {
            self_handle,
            main_menu,
            back: ui.find_handle_by_name_from_root("SVBack"),
            players_list: ui.find_handle_by_name_from_root("SVPlayersList"),
            start: ui.find_handle_by_name_from_root("SVStart"),
            server_address_input: ui.find_handle_by_name_from_root("SVServerAddress"),
            add_bots_check_box: ui.find_handle_by_name_from_root("SVAddBotsCheckBox"),
            level_selector,
            server_address: "127.0.0.1:10001".to_string(),
            selected_level: available_levels.first().map(|_| 0),
            available_levels,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        ctx: &mut PluginContext,
        message: &UiMessage,
        server: &mut Option<Server>,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.start {
                ctx.user_interface.send_message(WidgetMessage::visibility(
                    self.self_handle,
                    MessageDirection::ToWidget,
                    false,
                ));

                if let Some(selected_level) = self.selected_level {
                    if let Some(server) = server.as_mut() {
                        server.start_game(&self.available_levels[selected_level]);
                    }
                }
            } else if message.destination() == self.back {
                ctx.user_interface.send_message(WidgetMessage::visibility(
                    self.self_handle,
                    MessageDirection::ToWidget,
                    false,
                ));
                ctx.user_interface.send_message(WidgetMessage::visibility(
                    self.main_menu,
                    MessageDirection::ToWidget,
                    true,
                ));
                *server = None;
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.server_address_input
                && message.direction() == MessageDirection::FromWidget
            {
                self.server_address = text.clone();
            }
        } else if let Some(SelectorMessage::Current(selected)) = message.data() {
            if message.destination() == self.level_selector
                && message.direction() == MessageDirection::FromWidget
            {
                self.selected_level = *selected;
            }
        } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.add_bots_check_box
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(server) = server {
                    server.add_bots = *value;
                }
            }
        }
    }

    pub fn update(&self, ctx: &mut PluginContext, server: &Option<Server>) {
        let Some(server) = server else {
            return;
        };

        let player_entries_count = ctx
            .user_interface
            .node(self.players_list)
            .query_component::<ListView>()
            .unwrap()
            .items()
            .len();
        if server.connections().len() != player_entries_count {
            let new_player_entries = server
                .connections()
                .iter()
                .enumerate()
                .map(|(n, e)| {
                    make_text_widget(
                        &mut ctx.user_interface.build_ctx(),
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
            ctx.user_interface.send_message(ListViewMessage::items(
                self.players_list,
                MessageDirection::ToWidget,
                new_player_entries,
            ));
        }
    }
}

pub struct SettingsMenu {
    menu: Handle<UiNode>,
    graphics_quality: Handle<UiNode>,
    sound_volume: Handle<UiNode>,
    music_volume: Handle<UiNode>,
    mouse_sens: Handle<UiNode>,
    mouse_smoothness: Handle<UiNode>,
    back: Handle<UiNode>,
    reset: Handle<UiNode>,
}

impl SettingsMenu {
    pub fn new(
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        settings: &Settings,
    ) -> Self {
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

        let graphics_quality = ui.find_handle_by_name_from_root("SettingsGraphicsQuality");

        ui.send_message(SelectorMessage::set_items(
            graphics_quality,
            MessageDirection::ToWidget,
            items,
            true,
        ));
        ui.send_message(SelectorMessage::current(
            graphics_quality,
            MessageDirection::ToWidget,
            Some(settings.graphics_quality),
        ));

        let sound_volume = ui.find_handle_by_name_from_root("SettingsSoundVolume");
        let music_volume = ui.find_handle_by_name_from_root("SettingsMusicVolume");
        let mouse_sens = ui.find_handle_by_name_from_root("SettingsMouseSens");
        let mouse_smoothness = ui.find_handle_by_name_from_root("SettingsMouseSmooth");

        fn set_sb_value(ui: &UserInterface, handle: Handle<UiNode>, value: f32) {
            ui.send_message(ScrollBarMessage::value(
                handle,
                MessageDirection::ToWidget,
                value,
            ));
        }
        set_sb_value(ui, sound_volume, settings.sound_volume);
        set_sb_value(ui, music_volume, settings.music_volume);
        set_sb_value(ui, mouse_sens, settings.mouse_sensitivity);
        set_sb_value(ui, mouse_smoothness, settings.mouse_smoothness);

        Self {
            menu: ui.find_handle_by_name_from_root("SettingsMenu"),
            graphics_quality,
            sound_volume,
            music_volume,
            mouse_sens,
            mouse_smoothness,
            back: ui.find_handle_by_name_from_root("SettingsBack"),
            reset: ui.find_handle_by_name_from_root("SettingsReset"),
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        main_menu: Handle<UiNode>,
        ui: &UserInterface,
        graphics_context: &mut GraphicsContext,
        settings: &mut Settings,
        scenes: &SceneContainer,
        game_scene: Handle<Scene>,
    ) {
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
                if let Some(scene) = scenes.try_get(game_scene) {
                    settings.apply_sound_volume(scene);
                }
            } else if message.destination() == self.music_volume {
                settings.write().music_volume = *value;
            } else if message.destination() == self.mouse_sens {
                settings.write().mouse_sensitivity = *value;
            } else if message.destination() == self.mouse_smoothness {
                settings.write().mouse_smoothness = *value;
            }
        }
    }
}

pub struct Menu {
    debug_text: Handle<UiNode>,
    settings: Handle<UiNode>,
    exit: Handle<UiNode>,
    start_as_server: Handle<UiNode>,
    start_as_client: Handle<UiNode>,
    main_menu: Handle<UiNode>,
    main_menu_root: Handle<UiNode>,
    background: Handle<UiNode>,
    server_menu: ServerMenu,
    settings_menu: SettingsMenu,
    scene: Handle<Scene>,
    click_begin_sound: Handle<Node>,
    click_end_sound: Handle<Node>,
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
    pub fn new(ctx: &mut PluginContext, settings: &Settings) -> Self {
        let mut scene = Scene::new();
        let click_begin_sound = SoundBuilder::new(BaseBuilder::new())
            .with_buffer(Some(
                ctx.resource_manager
                    .request::<SoundBuffer>("data/sound/click_begin.ogg"),
            ))
            .with_spatial_blend_factor(0.0)
            .build(&mut scene.graph);
        let click_end_sound = SoundBuilder::new(BaseBuilder::new())
            .with_buffer(Some(
                ctx.resource_manager
                    .request::<SoundBuffer>("data/sound/click_end.ogg"),
            ))
            .with_spatial_blend_factor(0.0)
            .build(&mut scene.graph);
        let scene = ctx.scenes.add(scene);

        let ui = &mut *ctx.user_interface;
        let main_menu = ui.find_handle_by_name_from_root("MainMenu");
        let server_menu = ui.find_handle_by_name_from_root("ServerMenu");
        Self {
            debug_text: ui.find_handle_by_name_from_root("DebugText"),
            settings: ui.find_handle_by_name_from_root("Settings"),
            exit: ui.find_handle_by_name_from_root("Exit"),
            start_as_server: ui.find_handle_by_name_from_root("Server"),
            start_as_client: ui.find_handle_by_name_from_root("Client"),
            main_menu,
            main_menu_root: ui.find_handle_by_name_from_root("MainMenuRoot"),
            background: ui.find_handle_by_name_from_root("Background"),
            server_menu: ServerMenu::new(server_menu, main_menu, ui, ctx.resource_manager),
            settings_menu: SettingsMenu::new(ui, ctx.resource_manager, settings),
            scene,
            click_begin_sound,
            click_end_sound,
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
    ) {
        self.server_menu.handle_ui_message(ctx, message, server);
        self.settings_menu.handle_ui_message(
            message,
            self.main_menu,
            ctx.user_interface,
            ctx.graphics_context,
            settings,
            ctx.scenes,
            game_scene,
        );

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.exit {
                if let Some(window_target) = ctx.window_target {
                    window_target.exit();
                }
            } else if message.destination() == self.start_as_server {
                set_visibility(
                    ctx.user_interface,
                    &[
                        (self.server_menu.self_handle, true),
                        (self.main_menu, false),
                    ],
                );
                ctx.user_interface.send_message(TextMessage::text(
                    self.server_menu.server_address_input,
                    MessageDirection::ToWidget,
                    Server::LOCALHOST.to_string(),
                ));

                // Try to start the server and the client.
                match Server::new(&self.server_menu.server_address) {
                    Ok(new_server) => {
                        *server = Some(new_server);
                        *client = try_connect_to_server(&self.server_menu.server_address);
                        let server = server.as_mut().unwrap();
                        server.accept_connections();
                    }
                    Err(err) => Log::err(format!("Unable to create a server. Reason: {:?}", err)),
                }
            } else if message.destination() == self.start_as_client {
                *client = try_connect_to_server(&self.server_menu.server_address);
            } else if message.destination() == self.settings {
                set_visibility(
                    ctx.user_interface,
                    &[(self.settings_menu.menu, true), (self.main_menu, false)],
                );
            }
        }

        if let Some(scene) = ctx.scenes.try_get_mut(self.scene) {
            let graph = &mut scene.graph;
            if let Some(WidgetMessage::MouseDown { .. }) = message.data() {
                utils::try_play_sound(self.click_begin_sound, graph);
            } else if let Some(WidgetMessage::MouseUp { .. }) = message.data() {
                utils::try_play_sound(self.click_end_sound, graph);
            }
        }
    }

    pub fn set_main_menu_visibility(&self, ui: &UserInterface, visible: bool) {
        ui.send_message(WidgetMessage::visibility(
            self.main_menu_root,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    pub fn switch_visibility(&self, ui: &UserInterface, is_client_running: bool) {
        let is_visible = ui.node(self.main_menu_root).is_globally_visible();
        set_visibility(
            ui,
            &[
                (self.main_menu_root, !is_visible),
                (self.main_menu, !is_visible),
                (self.server_menu.self_handle, false),
                (self.background, !is_client_running),
            ],
        );
    }

    pub fn is_active(&self, ui: &UserInterface) -> bool {
        ui.try_get(self.main_menu_root)
            .map(|n| n.is_globally_visible())
            .unwrap_or_default()
    }

    pub fn update(&self, ctx: &mut PluginContext, server: &Option<Server>) {
        self.server_menu.update(ctx, server);

        if let GraphicsContext::Initialized(graphics_context) = ctx.graphics_context {
            let fps = graphics_context.renderer.get_statistics().frames_per_second;
            ctx.user_interface.send_message(TextMessage::text(
                self.debug_text,
                MessageDirection::ToWidget,
                format!("FPS: {fps}"),
            ));
        }
    }
}
