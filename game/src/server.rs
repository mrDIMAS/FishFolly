use crate::net::{ClientMessage, NetSocket};
use fyrox::core::log::Log;
use std::{io::ErrorKind, net::SocketAddr};

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

    pub fn read_messages(&mut self) {
        loop {
            let mut bytes = [0; 8192];
            match self.socket.recv_from(&mut bytes) {
                Ok((bytes_count, sender_address)) => {
                    if bytes_count == 0 {
                        break;
                    } else {
                        let received_data = &bytes[..bytes_count];
                        if let Some(message) = ClientMessage::try_create(received_data) {
                            match message {
                                ClientMessage::Connect { name } => {
                                    Log::info(format!("Client {} connected successfully!", name));

                                    self.clients.push(sender_address);
                                }
                            }
                        } else {
                            Log::err("Malformed server message!");
                        }
                    }
                }
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    }
                    ErrorKind::Interrupted => {
                        // Retry
                    }
                    _ => Log::err(format!(
                        "An error occurred when reading data from socket: {}",
                        err
                    )),
                },
            }
        }
    }
}
