//! Object marker components.

use crate::{utils, Game};
use fyrox::{
    core::{algebra::Vector3, pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{graph::Graph, node::Node, ragdoll::Ragdoll, rigidbody::RigidBody},
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
    fn is_ragdoll_has_ground_contact(&self, graph: &Graph) -> bool {
        let mut result = false;
        if let Some(ragdoll) = graph.try_get_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.root_limb().iterate_recursive(&mut |limb| {
                if let Some(rigid_body) = graph.try_get_of_type::<RigidBody>(limb.physical_bone) {
                    for child in rigid_body.children() {
                        if utils::has_ground_contact(*child, graph) {
                            result = true;
                            break;
                        }
                    }
                }
            });
        }
        result
    }

    pub fn has_ground_contact(&self, graph: &Graph) -> bool {
        utils::has_ground_contact(self.collider, graph) || self.is_ragdoll_has_ground_contact(graph)
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

    pub fn for_each_rigid_body<F>(&mut self, graph: &mut Graph, mut func: F)
    where
        F: FnMut(&mut RigidBody),
    {
        let mut mbc = graph.begin_multi_borrow::<32>();
        if let Some(rigid_body) = mbc
            .try_get(self.rigid_body)
            .and_then(|n| n.query_component_mut::<RigidBody>())
        {
            func(rigid_body)
        }
        if let Some(ragdoll) = mbc
            .try_get(self.ragdoll)
            .and_then(|n| n.query_component_ref::<Ragdoll>())
        {
            ragdoll.root_limb().iterate_recursive(&mut |limb| {
                if let Some(rigid_body) = mbc
                    .try_get(limb.physical_bone)
                    .and_then(|n| n.query_component_mut::<RigidBody>())
                {
                    func(rigid_body)
                }
            });
        }
    }

    pub fn set_velocity(&mut self, velocity: Vector3<f32>, graph: &mut Graph, xz_plane_only: bool) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            let y_vel = rigid_body.lin_vel().y;
            rigid_body.set_lin_vel(if xz_plane_only {
                Vector3::new(velocity.x, y_vel, velocity.z)
            } else {
                velocity
            });
        });
    }

    pub fn add_force(&mut self, force: Vector3<f32>, graph: &mut Graph) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            rigid_body.apply_force(force);
        });
    }

    pub fn do_move(&mut self, velocity: Vector3<f32>, graph: &mut Graph, has_ground_contact: bool) {
        if has_ground_contact {
            self.set_velocity(velocity, graph, !self.jump);
        } else {
            self.add_force(velocity.scale(0.75), graph);
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
