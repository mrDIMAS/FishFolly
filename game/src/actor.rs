//! Object marker components.

use fyrox::{
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{graph::Graph, node::Node, ragdoll::Ragdoll},
    script::ScriptContext,
};

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
    #[reflect(description = "Handle to player's collider.")]
    pub collider: Handle<Node>,
}

impl Default for Actor {
    fn default() -> Self {
        Self {
            stand_up_timer: 0.0,
            stand_up_timeout: 2.0,
            ragdoll: Default::default(),
            jump: false,
            collider: Default::default(),
        }
    }
}

impl Actor {
    pub fn set_ragdoll_enabled(&self, graph: &mut Graph, enabled: bool) {
        if let Some(ragdoll) = graph.try_get_mut_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.set_active(enabled);
        }
    }

    pub fn on_update(&mut self, ctx: &mut ScriptContext) {
        self.stand_up_timer -= ctx.dt;
        if self.jump && self.stand_up_timer <= 0.0 {
            self.set_ragdoll_enabled(&mut ctx.scene.graph, true);
            self.stand_up_timer = self.stand_up_timeout;
        }
        if self.stand_up_timer <= 0.0 {
            self.set_ragdoll_enabled(&mut ctx.scene.graph, false);
        }
        self.jump = false;
    }
}
