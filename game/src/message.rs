use fyrox::{core::pool::Handle, scene::node::Node};

pub enum Message {
    UnregisterTarget(Handle<Node>),
    UnregisterActor(Handle<Node>),
    UnregisterStartPoint(Handle<Node>),
}
