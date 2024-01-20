use crate::net::{ClientMessage, NetSocket};
use fyrox::core::log::Log;
use std::{fmt::Debug, net::ToSocketAddrs};

pub struct Client {
    socket: NetSocket,
}

impl Client {
    pub fn new() -> Self {
        Self {
            socket: NetSocket::bind("127.0.0.1:10000").unwrap(),
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
}
