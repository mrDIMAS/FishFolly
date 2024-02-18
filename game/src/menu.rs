use crate::{client::Client, server::Server, Game};
use fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle},
    graph::{BaseSceneGraph, SceneGraph},
    gui::{
        button::ButtonMessage,
        check_box::CheckBoxMessage,
        font::Font,
        list_view::{ListView, ListViewMessage},
        message::{MessageDirection, UiMessage},
        selector::SelectorMessage,
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    plugin::PluginContext,
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

pub struct Menu {
    debug_text: Handle<UiNode>,
    settings: Handle<UiNode>,
    exit: Handle<UiNode>,
    start_as_server: Handle<UiNode>,
    start_as_client: Handle<UiNode>,
    main_menu: Handle<UiNode>,
    main_menu_root: Handle<UiNode>,
    server_menu: ServerMenu,
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
    pub fn new(ctx: &mut PluginContext) -> Self {
        ctx.task_pool.spawn_plugin_task(
            UserInterface::load_from_file("data/menu.ui", ctx.resource_manager.clone()),
            |result, game: &mut Game, ctx| {
                *ctx.user_interface = result.unwrap();
                let menu = &mut game.menu;
                let ui = &mut *ctx.user_interface;
                menu.exit = ui.find_handle_by_name_from_root("Exit");
                menu.debug_text = ui.find_handle_by_name_from_root("DebugText");
                menu.start_as_server = ui.find_handle_by_name_from_root("Server");
                menu.start_as_client = ui.find_handle_by_name_from_root("Client");
                menu.main_menu = ui.find_handle_by_name_from_root("MainMenu");
                menu.settings = ui.find_handle_by_name_from_root("Settings");
                menu.main_menu_root = ui.find_handle_by_name_from_root("MainMenuRoot");
                let server_menu = ui.find_handle_by_name_from_root("ServerMenu");
                menu.server_menu =
                    ServerMenu::new(server_menu, menu.main_menu, ui, ctx.resource_manager);
            },
        );
        Self {
            debug_text: Default::default(),
            settings: Default::default(),
            exit: Default::default(),
            start_as_server: Default::default(),
            start_as_client: Default::default(),
            main_menu: Default::default(),
            main_menu_root: Default::default(),
            server_menu: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        ctx: &mut PluginContext,
        message: &UiMessage,
        server: &mut Option<Server>,
        client: &mut Option<Client>,
    ) {
        self.server_menu.handle_ui_message(ctx, message, server);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.exit {
                if let Some(window_target) = ctx.window_target {
                    window_target.exit();
                }
            } else if message.destination() == self.start_as_server {
                ctx.user_interface.send_message(WidgetMessage::visibility(
                    self.server_menu.self_handle,
                    MessageDirection::ToWidget,
                    true,
                ));
                ctx.user_interface.send_message(WidgetMessage::visibility(
                    self.main_menu,
                    MessageDirection::ToWidget,
                    false,
                ));
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

    pub fn switch_main_menu_visibility(&self, ui: &UserInterface) {
        let is_visible = ui.node(self.main_menu_root).is_globally_visible();
        ui.send_message(WidgetMessage::visibility(
            self.main_menu_root,
            MessageDirection::ToWidget,
            !is_visible,
        ));
    }

    pub fn update(&self, ctx: &mut PluginContext, server: &Option<Server>) {
        self.server_menu.update(ctx, server)
    }
}
