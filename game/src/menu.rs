use crate::{client::Client, server::Server, Game};
use fyrox::graph::SceneGraph;
use fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle},
    gui::{
        button::ButtonMessage,
        font::Font,
        list_view::{ListView, ListViewMessage},
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        text::TextMessage,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    plugin::PluginContext,
};

pub fn make_player_entry(
    ctx: &mut BuildContext,
    name: &str,
    resource_manager: &ResourceManager,
) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
        .with_vertical_text_alignment(VerticalAlignment::Center)
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
    server_address: Handle<UiNode>,
}

impl ServerMenu {
    pub fn new(self_handle: Handle<UiNode>, main_menu: Handle<UiNode>, ui: &UserInterface) -> Self {
        Self {
            self_handle,
            main_menu,
            back: ui.find_handle_by_name_from_root("SVBack"),
            players_list: ui.find_handle_by_name_from_root("SVPlayersList"),
            start: ui.find_handle_by_name_from_root("SVStart"),
            server_address: ui.find_handle_by_name_from_root("SVServerAddress"),
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

                if let Some(server) = server.as_mut() {
                    server.start_game();
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
                .map(|e| {
                    make_player_entry(
                        &mut ctx.user_interface.build_ctx(),
                        &e.string_peer_address(),
                        ctx.resource_manager,
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
    single_player: Handle<UiNode>,
    settings: Handle<UiNode>,
    exit: Handle<UiNode>,
    start_as_server: Handle<UiNode>,
    start_as_client: Handle<UiNode>,
    main_menu: Handle<UiNode>,
    main_menu_root: Handle<UiNode>,
    server_menu: ServerMenu,
}

fn try_connect_to_server() -> Option<Client> {
    match Client::try_connect(Server::ADDRESS) {
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
                menu.single_player = ui.find_handle_by_name_from_root("SinglePlayer");
                menu.exit = ui.find_handle_by_name_from_root("Exit");
                menu.debug_text = ui.find_handle_by_name_from_root("DebugText");
                menu.start_as_server = ui.find_handle_by_name_from_root("Server");
                menu.start_as_client = ui.find_handle_by_name_from_root("Client");
                menu.main_menu = ui.find_handle_by_name_from_root("MainMenu");
                menu.settings = ui.find_handle_by_name_from_root("Settings");
                menu.main_menu_root = ui.find_handle_by_name_from_root("MainMenuRoot");
                let server_menu = ui.find_handle_by_name_from_root("ServerMenu");
                menu.server_menu = ServerMenu::new(server_menu, menu.main_menu, ui);
            },
        );
        Self {
            debug_text: Default::default(),
            single_player: Default::default(),
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
                    self.server_menu.server_address,
                    MessageDirection::ToWidget,
                    Server::ADDRESS.to_string(),
                ));

                // Try to start the server and the client.
                match Server::new() {
                    Ok(new_server) => {
                        *server = Some(new_server);
                        *client = try_connect_to_server();
                        let server = server.as_mut().unwrap();
                        server.accept_connections();
                    }
                    Err(err) => Log::err(format!("Unable to create a server. Reason: {:?}", err)),
                }
            } else if message.destination() == self.start_as_client {
                *client = try_connect_to_server();
            } else if message.destination() == self.single_player {
                match Server::new() {
                    Ok(new_server) => {
                        *server = Some(new_server);
                        *client = try_connect_to_server();
                        let server = server.as_mut().unwrap();
                        server.accept_connections();
                        server.start_game();
                    }
                    Err(err) => Log::err(format!("Unable to create a server. Reason: {:?}", err)),
                }
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
