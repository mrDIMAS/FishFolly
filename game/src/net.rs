use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        byteorder::{LittleEndian, WriteBytesExt},
        log::Log,
        pool::Handle,
    },
    scene::node::Node,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::PathBuf,
};

pub trait Message: Sized + Debug {
    fn try_create(bytes: &[u8]) -> Result<Self, bincode::Error>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeState {
    pub node: Handle<Node>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceDescriptor {
    pub path: PathBuf,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub velocity: Vector3<f32>, // Rigid body only.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerDescriptor {
    pub path: PathBuf,
    pub position: Vector3<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTickMessage {
    pub nodes: Vec<NodeState>,
}

/// A message sent from the server to a client.
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    LoadLevel { path: PathBuf },
    UpdateTick(UpdateTickMessage),
    AddPlayers(Vec<PlayerDescriptor>),
    Instantiate(Vec<InstanceDescriptor>),
}

impl Message for ServerMessage {
    fn try_create(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// A message sent from a client to the server.
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Connect { name: String },
}

impl Message for ClientMessage {
    fn try_create(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

pub struct NetListener {
    listener: TcpListener,
}

impl NetListener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    pub fn accept_connections(&self) -> Vec<NetStream> {
        let mut streams = Vec::new();
        while let Ok(result) = self.listener.accept() {
            streams.push(NetStream::from_inner(result.0).unwrap())
        }
        streams
    }
}

pub struct NetStream {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl NetStream {
    pub fn from_inner(stream: TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            buffer: Default::default(),
        })
    }

    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Self::from_inner(TcpStream::connect(addr)?)
    }

    pub fn send_message<T>(&mut self, data: &T) -> io::Result<()>
    where
        T: Serialize,
    {
        let data = bincode::serialize(data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        self.stream.write_u32::<LittleEndian>(data.len() as u32)?;
        self.stream.write_all(&data)?;
        Ok(())
    }

    pub fn peer_address(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    pub fn string_peer_address(&self) -> String {
        self.peer_address()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "Unknown".into())
    }

    fn next_message<M: Message>(&mut self) -> Option<M> {
        if self.buffer.len() < 4 {
            return None;
        }

        let length = u32::from_le_bytes([
            self.buffer[0],
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
        ]) as usize;

        let end = 4 + length;
        let message = match M::try_create(&self.buffer[4..end]) {
            Ok(message) => Some(message),
            Err(err) => {
                Log::err(format!(
                    "Failed to parse a network message of {} bytes long. Reason: {:?}",
                    length, err
                ));

                None
            }
        };

        self.buffer.drain(..end);

        message
    }

    pub fn process_input<M>(&mut self, mut func: impl FnMut(M))
    where
        M: Message,
    {
        // Receive all bytes from the stream first.
        loop {
            let mut bytes = [0; 8192];
            match self.stream.read(&mut bytes) {
                Ok(bytes_count) => {
                    if bytes_count == 0 {
                        break;
                    } else {
                        self.buffer.extend(&bytes[..bytes_count])
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

        // Extract all the messages and process them.
        while let Some(message) = self.next_message() {
            func(message)
        }
    }
}
