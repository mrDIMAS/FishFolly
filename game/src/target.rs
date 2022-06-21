//! A target that bots will try to reach.

use crate::{game_mut, Game, Message};
use fyrox::{
    core::{inspect::prelude::*, pool::Handle, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    impl_component_provider,
    scene::{node::Node, node::TypeUuidProvider},
    script::{ScriptContext, ScriptTrait},
};
use std::sync::mpsc::Sender;

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct Target {
    #[visit(skip)]
    #[inspect(skip)]
    self_handle: Handle<Node>,
    #[visit(skip)]
    #[inspect(skip)]
    sender: Option<Sender<Message>>,
}

impl_component_provider!(Target);

impl TypeUuidProvider for Target {
    fn type_uuid() -> Uuid {
        uuid!("dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.as_ref() {
            sender
                .send(Message::UnregisterTarget(self.self_handle))
                .unwrap();
        }
    }
}

impl ScriptTrait for Target {
    fn on_init(&mut self, context: ScriptContext) {
        let game = game_mut(context.plugin);
        self.self_handle = context.handle;
        self.sender = Some(game.message_sender.clone());
        game.targets.insert(context.handle);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
