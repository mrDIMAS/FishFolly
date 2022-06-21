//! Object marker components.

use crate::Message;
use fyrox::{core::pool::Handle, scene::node::Node};
use std::sync::mpsc::Sender;

/// A marker that indicates that an object is an actor (player or bot).
#[derive(Clone, Default, Debug)]
pub struct Actor {
    pub self_handle: Handle<Node>,
    pub sender: Option<Sender<Message>>,
}

impl Drop for Actor {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.as_ref() {
            sender
                .send(Message::UnregisterActor(self.self_handle))
                .unwrap();
        }
    }
}
