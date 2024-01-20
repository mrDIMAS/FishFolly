use crate::net::{ClientMessage, NetListener, NetStream, ServerMessage};
use fyrox::core::log::Log;
use std::io;

pub struct Server {
    listener: NetListener,
    connections: Vec<NetStream>,
}

impl Server {
    pub const ADDRESS: &'static str = "127.0.0.1:10001"; // TODO

    pub fn new() -> io::Result<Self> {
        Ok(Self {
            listener: NetListener::bind(Self::ADDRESS)?,
            connections: Default::default(),
        })
    }

    pub fn start_game(&mut self) {
        for client in self.connections.iter_mut() {
            Log::verify(client.send_message(&ServerMessage::LoadLevel {
                path: "data/drake.rgs".into(),
            }))
        }
    }

    pub fn read_messages(&mut self) {
        for player in self.connections.iter_mut() {
            player.process_input::<ClientMessage>(|msg| match msg {
                ClientMessage::Connect { name } => {
                    Log::info(format!("Client {} connected successfully!", name));
                }
            });
        }
    }

    pub fn connections(&self) -> &[NetStream] {
        &self.connections
    }

    pub fn accept_connections(&mut self) {
        self.connections.extend(self.listener.accept_connections())
    }
}
