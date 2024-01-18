//! Object marker components.

use crate::{utils, Game};
use fyrox::{
    core::{algebra::Vector3, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{graph::Graph, node::Node, ragdoll::Ragdoll},
    script::{ScriptContext, ScriptMessageContext, ScriptMessagePayload},
};

#[derive(Debug)]
pub enum ActorMessage {
    RespawnAt(Vector3<f32>),
}

/// A marker that indicates that an object is an actor (player or bot).
#[derive(Clone, Debug, Visit, Reflect)]
#[visit(optional)]
pub struct Actor {
    #[visit(skip)]
    #[reflect(hidden)]
    pub stand_up_timer: f32,
    #[reflect(
        description = "Amount of time that the bot will be lying on the ground with active ragdoll."
    )]
    stand_up_timeout: f32,
    #[reflect(description = "A handle of the ragdoll")]
    ragdoll: Handle<Node>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub jump: bool,
    #[reflect(description = "Handle to actor's collider.")]
    pub collider: Handle<Node>,
    #[reflect(description = "Handle to actor's rigid body.")]
    pub rigid_body: Handle<Node>,
}

impl Default for Actor {
    fn default() -> Self {
        Self {
            stand_up_timer: 0.0,
            stand_up_timeout: 2.0,
            ragdoll: Default::default(),
            jump: false,
            collider: Default::default(),
            rigid_body: Default::default(),
        }
    }
}

impl Actor {
    pub fn has_ground_contact(&self, graph: &Graph) -> bool {
        utils::has_ground_contact(self.collider, graph)
    }

    pub fn set_ragdoll_enabled(&mut self, graph: &mut Graph, enabled: bool) {
        if let Some(ragdoll) = graph.try_get_mut_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.set_active(enabled);
        }
        self.stand_up_timer = 0.0;
    }

    pub fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) {
        let Some(message) = message.downcast_ref::<ActorMessage>() else {
            return;
        };

        match message {
            ActorMessage::RespawnAt(position) => {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, false);

                if let Some(rigid_body) = ctx.scene.graph.try_get_mut(self.rigid_body) {
                    rigid_body.local_transform_mut().set_position(*position);
                }
            }
        }
    }

    pub fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        let has_ground_contact = self.has_ground_contact(&ctx.scene.graph);
        self.stand_up_timer -= ctx.dt;
        if !game.debug_settings.disable_ragdoll && self.jump && self.stand_up_timer <= 0.0 {
            self.set_ragdoll_enabled(&mut ctx.scene.graph, true);
            self.stand_up_timer = self.stand_up_timeout;
        }
        if self.stand_up_timer <= 0.0 && has_ground_contact {
            self.set_ragdoll_enabled(&mut ctx.scene.graph, false);
        }
        self.jump = false;
    }
}
