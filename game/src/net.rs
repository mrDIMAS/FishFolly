use serde::{Deserialize, Serialize};
use std::{
    io,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

/// A message sent from the server to a client.
#[derive(Serialize, Deserialize)]
pub enum ServerMessage {}

impl ServerMessage {
    pub fn try_create(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
}

/// A message sent from a client to the server.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { name: String },
}

impl ClientMessage {
    pub fn try_create(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
}

pub struct NetSocket {
    socket: UdpSocket,
}

impl NetSocket {
    pub fn bind<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        self.socket.connect(addr)
    }

    pub fn send<T>(&self, data: &T) -> io::Result<()>
    where
        T: Serialize,
    {
        let data = bincode::serialize(data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.socket.send(&data)?;
        Ok(())
    }

    pub fn send_to<T, A>(&self, data: &T, addr: A) -> io::Result<()>
    where
        T: Serialize,
        A: ToSocketAddrs,
    {
        let data = bincode::serialize(data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.socket.send_to(&data, addr)?;
        Ok(())
    }

    pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buf)
    }
}
