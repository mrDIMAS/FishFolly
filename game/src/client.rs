use crate::net::{ClientMessage, NetSocket, ServerMessage};
use fyrox::{
    core::log::Log,
    plugin::PluginContext,
    rand::{thread_rng, Rng},
};
use std::{
    fmt::Debug,
    net::ToSocketAddrs,
    net::{IpAddr, SocketAddr},
};

pub struct Client {
    socket: NetSocket,
}

impl Client {
    pub fn new() -> Self {
        let addr = SocketAddr::new(
            IpAddr::from([127, 0, 0, 1]),
            thread_rng().gen_range(1024..65530),
        );
        Self {
            socket: NetSocket::bind(addr).unwrap(),
        }
    }

    pub fn send_message(&self, message: ClientMessage) {
        match self.socket.send(&message) {
            Ok(_) => {}
            Err(err) => Log::err(format!("Unable to send client message: {}", err)),
        }
    }

    pub fn try_connect<A>(&self, addr: A)
    where
        A: ToSocketAddrs + Debug,
    {
        match self.socket.connect(&addr) {
            Ok(_) => {
                Log::info(format!("Successfully connected to: {:?}", addr));

                self.send_message(ClientMessage::Connect {
                    name: "Foobar".into(),
                })
            }
            Err(err) => Log::err(format!(
                "An error occurred when trying to connect to the server: {}",
                err
            )),
        }
    }

    pub fn read_messages(&mut self, context: &mut PluginContext) {
        self.socket.process_input(|data, sender_address| {
            if let Some(message) = ServerMessage::try_create(data) {
                match message {
                    ServerMessage::LoadLevel { path } => {
                        context.async_scene_loader.request(path);
                    }
                }
            } else {
                Log::err("Malformed server message!");
            }
        })
    }
}
