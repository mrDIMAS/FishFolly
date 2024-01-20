use crate::net::{ClientMessage, NetStream, ServerMessage};
use fyrox::{core::log::Log, plugin::PluginContext};
use std::{fmt::Debug, io, net::ToSocketAddrs};

pub struct Client {
    connection: NetStream,
}

impl Client {
    pub fn try_connect<A>(server_addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs + Debug,
    {
        Ok(Self {
            connection: NetStream::connect(server_addr)?,
        })
    }

    pub fn send_message(&mut self, message: ClientMessage) {
        match self.connection.send_message(&message) {
            Ok(_) => {}
            Err(err) => Log::err(format!("Unable to send client message: {}", err)),
        }
    }

    pub fn read_messages(&mut self, context: &mut PluginContext) {
        self.connection.process_input(|msg| match msg {
            ServerMessage::LoadLevel { path } => {
                context.async_scene_loader.request(path);
            }
        })
    }
}
