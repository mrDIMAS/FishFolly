use crate::net::{ClientMessage, NetSocket, ServerMessage};
use fyrox::core::log::Log;
use std::net::SocketAddr;

pub struct Server {
    socket: NetSocket,
    clients: Vec<SocketAddr>,
}

impl Server {
    pub const ADDRESS: &'static str = "127.0.0.1:10001"; // TODO

    pub fn new() -> Self {
        Self {
            socket: NetSocket::bind(Self::ADDRESS).unwrap(),
            clients: Default::default(),
        }
    }

    pub fn start_game(&self) {
        for client in self.clients.iter() {
            Log::verify(self.socket.send_to(
                &ServerMessage::LoadLevel {
                    path: "data/drake.rgs".into(),
                },
                client,
            ))
        }
    }

    pub fn read_messages(&mut self) {
        self.socket.process_input(|data, sender_address| {
            if let Some(message) = ClientMessage::try_create(data) {
                match message {
                    ClientMessage::Connect { name } => {
                        Log::info(format!("Client {} connected successfully!", name));

                        self.clients.push(sender_address);
                    }
                }
            } else {
                Log::err("Malformed server message!");
            }
        })
    }
}
