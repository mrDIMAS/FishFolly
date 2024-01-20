use crate::{client::Client, server::Server, Game};
use fyrox::{
    asset::io::FsResourceIo,
    core::pool::Handle,
    gui::{
        button::ButtonMessage,
        constructor::WidgetConstructorContainer,
        message::{MessageDirection, UiMessage},
        widget::WidgetMessage,
        UiNode, UserInterface,
    },
    plugin::PluginContext,
};
use std::{path::Path, sync::Arc};

pub struct Menu {
    debug_text: Handle<UiNode>,
    new_game: Handle<UiNode>,
    exit: Handle<UiNode>,
    start_as_server: Handle<UiNode>,
    start_as_client: Handle<UiNode>,
}

impl Menu {
    pub fn new(ctx: &mut PluginContext) -> Self {
        ctx.task_pool.spawn_plugin_task(
            UserInterface::load_from_file(
                Path::new("data/menu.ui"),
                Arc::new(WidgetConstructorContainer::new()),
                ctx.resource_manager.clone(),
                &FsResourceIo,
            ),
            |result, game: &mut Game, ctx| {
                *ctx.user_interface = result.unwrap();
                game.menu.new_game = ctx.user_interface.find_by_name_down_from_root("NewGame");
                game.menu.exit = ctx.user_interface.find_by_name_down_from_root("Exit");
                game.menu.debug_text = ctx.user_interface.find_by_name_down_from_root("DebugText");
                game.menu.start_as_server =
                    ctx.user_interface.find_by_name_down_from_root("Server");
                game.menu.start_as_client =
                    ctx.user_interface.find_by_name_down_from_root("Client");
            },
        );
        Self {
            debug_text: Default::default(),
            new_game: Default::default(),
            exit: Default::default(),
            start_as_server: Default::default(),
            start_as_client: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        ctx: &mut PluginContext,
        message: &UiMessage,
        server: &mut Option<Server>,
        client: &mut Client,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.new_game {
                if let Some(server) = server.as_ref() {
                    server.start_game();
                }

                ctx.user_interface.send_message(WidgetMessage::visibility(
                    ctx.user_interface.root(),
                    MessageDirection::ToWidget,
                    false,
                ));
            } else if message.destination() == self.exit {
                if let Some(window_target) = ctx.window_target {
                    window_target.exit();
                }
            } else if message.destination() == self.start_as_server {
                *server = Some(Server::new());
                client.try_connect(Server::ADDRESS);
            } else if message.destination() == self.start_as_client {
                client.try_connect(Server::ADDRESS);
            }
        }
    }
}
