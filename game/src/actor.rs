//! Object marker components.

use crate::{utils, Game};
use fyrox::{
    core::{
        algebra::Vector3, math::Vector3Ext, pool::Handle, pool::MultiBorrowContext,
        reflect::prelude::*, variable::InheritableVariable, visitor::prelude::*,
    },
    graph::SceneGraph,
    rand::{prelude::SliceRandom, thread_rng},
    resource::model::{ModelResource, ModelResourceExtension},
    scene::{
        animation::{absm::prelude::*, AnimationPlayer},
        collider::Collider,
        graph::Graph,
        node::{container::NodeContainer, Node},
        ragdoll::Ragdoll,
        rigidbody::RigidBody,
        sound::Sound,
    },
    script::{ScriptContext, ScriptMessageContext, ScriptMessagePayload},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, ScriptMessagePayload)]
pub enum ActorMessage {
    RespawnAt(Vector3<f32>),
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Visit, Serialize, Deserialize)]
pub enum ActorKind {
    Bot,
    Player,
    RemotePlayer,
}

/// A marker that indicates that an object is an actor (player or bot).
#[derive(Clone, Debug, Visit, Reflect)]
#[visit(optional)]
pub struct Actor {
    pub name: String,
    #[reflect(hidden)]
    pub kind: ActorKind,
    #[reflect(hidden)]
    pub in_air_time: f32,
    /// Amount of time that the bot will be lying on the ground with active ragdoll
    pub max_in_air_time: f32,
    #[reflect(hidden)]
    pub stand_up_timer: f32,
    #[reflect(hidden)]
    pub stand_up_interval: f32,
    /// A handle of the ragdoll
    pub ragdoll: Handle<Node>,
    #[reflect(hidden)]
    pub jump: bool,
    /// Handle to actor's collider.
    pub collider: Handle<Node>,
    /// Handle to actor's rigid body.
    pub rigid_body: Handle<Node>,
    /// Speed of the actor.
    pub speed: f32,
    /// Jump speed of the actor.
    pub jump_vel: f32,
    #[reflect(hidden)]
    pub target_desired_velocity: Vector3<f32>,
    #[reflect(hidden)]
    pub desired_velocity: Vector3<f32>,
    /// Handle of animation state machine.
    pub absm: Handle<Node>,
    #[reflect(hidden)]
    pub jump_interval: f32,
    pub footsteps: InheritableVariable<Vec<Handle<Node>>>,
    pub disappear_effect: InheritableVariable<Option<ModelResource>>,
    pub appear_effect: InheritableVariable<Option<ModelResource>>,
}

impl Default for Actor {
    fn default() -> Self {
        Self {
            name: "Player".to_string(),
            kind: ActorKind::Player,
            in_air_time: 0.0,
            max_in_air_time: 1.1,
            stand_up_timer: 0.0,
            stand_up_interval: 1.0,
            ragdoll: Default::default(),
            jump: false,
            collider: Default::default(),
            rigid_body: Default::default(),
            speed: 4.0,
            jump_vel: 6.5,
            target_desired_velocity: Default::default(),
            desired_velocity: Default::default(),
            absm: Default::default(),
            jump_interval: 0.0,
            footsteps: Default::default(),
            disappear_effect: Default::default(),
            appear_effect: Default::default(),
        }
    }
}

impl Actor {
    fn is_ragdoll_has_ground_contact(&self, graph: &Graph) -> bool {
        let mut result = false;
        if let Some(ragdoll) = graph.try_get_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.root_limb.iterate_recursive(&mut |limb| {
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
            ragdoll.is_active.set_value_and_mark_modified(enabled);
        }
    }

    pub fn is_ragdoll_enabled(&self, graph: &Graph) -> bool {
        if let Some(ragdoll) = graph.try_get_of_type::<Ragdoll>(self.ragdoll) {
            *ragdoll.is_active
        } else {
            false
        }
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
                if let Some(disappear_effect) = self.disappear_effect.as_ref() {
                    let current_position = ctx.scene.graph[self.rigid_body].global_position();
                    disappear_effect.instantiate_at(
                        ctx.scene,
                        current_position,
                        Default::default(),
                    );
                }

                self.set_ragdoll_enabled(&mut ctx.scene.graph, false);

                self.for_each_rigid_body(&mut ctx.scene.graph, |rb| {
                    rb.local_transform_mut().set_position(*position);
                });

                if let Some(appear_effect) = self.appear_effect.as_ref() {
                    appear_effect.instantiate_at(ctx.scene, *position, Default::default());
                }
            }
        }
    }

    pub fn for_each_rigid_body<F>(&mut self, graph: &mut Graph, mut func: F)
    where
        F: FnMut(&mut RigidBody),
    {
        let mbc = graph.begin_multi_borrow();
        if let Ok(mut rigid_body) = mbc.try_get_component_of_type_mut::<RigidBody>(self.rigid_body)
        {
            func(&mut rigid_body)
        }
        if let Ok(ragdoll) = mbc.try_get_component_of_type::<Ragdoll>(self.ragdoll) {
            ragdoll.root_limb.iterate_recursive(&mut |limb| {
                if let Ok(mut rigid_body) =
                    mbc.try_get_component_of_type_mut::<RigidBody>(limb.physical_bone)
                {
                    func(&mut rigid_body)
                }
            });
        };
    }

    pub fn set_velocity(&mut self, velocity: Vector3<f32>, graph: &mut Graph) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            let y_vel = rigid_body.lin_vel().y + velocity.y;
            rigid_body.set_lin_vel(Vector3::new(velocity.x, y_vel, velocity.z));
        });
    }

    pub fn add_force(&mut self, force: Vector3<f32>, max_speed: f32, graph: &mut Graph) {
        self.for_each_rigid_body(graph, &mut |rigid_body: &mut RigidBody| {
            if rigid_body.lin_vel().xz().norm() < max_speed {
                rigid_body.apply_force(force);
            }
        });
    }

    pub fn do_move(&mut self, velocity: Vector3<f32>, graph: &mut Graph) {
        if !self.is_ragdoll_enabled(graph) {
            self.set_velocity(velocity, graph);
        }
    }

    pub fn jump(&mut self) {
        if self.jump_interval <= 0.0 {
            self.jump_interval = 0.35;
            self.jump = true;
        }
    }

    fn has_serious_impact(&mut self, ctx: &mut ScriptContext) -> bool {
        if let Some(collider) = ctx.scene.graph.try_get_of_type::<Collider>(self.collider) {
            for contact in collider.contacts(&ctx.scene.graph.physics) {
                if contact.has_any_active_contact {
                    for manifold in contact.manifolds.iter() {
                        if let (Some(rb1), Some(rb2)) = (
                            ctx.scene
                                .graph
                                .try_get_of_type::<RigidBody>(manifold.rigid_body1),
                            ctx.scene
                                .graph
                                .try_get_of_type::<RigidBody>(manifold.rigid_body2),
                        ) {
                            if (rb1.lin_vel() - rb2.lin_vel()).norm() > 10.0
                                || manifold.points.iter().any(|p| p.impulse > 2.0)
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn is_in_jump_state(&self, graph: &Graph) -> bool {
        let name = "Jump";
        if let Some(absm) = graph.try_get_of_type::<AnimationBlendingStateMachine>(self.absm) {
            absm.machine().layers().first().map_or(false, |layer| {
                if let Some(active_state) = layer.states().try_borrow(layer.active_state()) {
                    active_state.name == name
                } else if let Some(active_transition) =
                    layer.transitions().try_borrow(layer.active_transition())
                {
                    if let Some(source) = layer.states().try_borrow(active_transition.source()) {
                        source.name == name
                    } else if let Some(dest) = layer.states().try_borrow(active_transition.dest()) {
                        dest.name == name
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    fn play_random_footstep_sound(&mut self, mbc: &MultiBorrowContext<Node, NodeContainer>) {
        let Some(random_footstep_sound) = self.footsteps.choose(&mut thread_rng()) else {
            return;
        };

        let Ok(mut sound) = mbc.try_get_component_of_type_mut::<Sound>(*random_footstep_sound)
        else {
            return;
        };

        sound.play();
    }

    fn process_animation_events(&mut self, ctx: &mut ScriptContext, has_ground_contact: bool) {
        let mbc = ctx.scene.graph.begin_multi_borrow();

        let Ok(absm) = mbc.try_get_component_of_type::<AnimationBlendingStateMachine>(self.absm)
        else {
            return;
        };

        let machine = absm.machine();

        let Ok(mut animation_player) =
            mbc.try_get_component_of_type_mut::<AnimationPlayer>(absm.animation_player())
        else {
            return;
        };

        let Some(first) = machine.layers().first() else {
            return;
        };

        let events_collection = first.collect_active_animations_events(
            machine.parameters(),
            animation_player.animations(),
            AnimationEventCollectionStrategy::All,
        );

        for (_, event) in events_collection.events {
            if event.name == "Footstep" && has_ground_contact {
                self.play_random_footstep_sound(&mbc);
            }
        }

        animation_player
            .animations_mut()
            .get_value_mut_silent()
            .clear_animation_events();
    }

    pub fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        let has_ground_contact = self.has_ground_contact(&ctx.scene.graph);
        if has_ground_contact {
            self.in_air_time = 0.0;
            self.stand_up_timer += ctx.dt;
            if self.stand_up_timer >= self.stand_up_interval {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, false);
            }
        } else {
            self.in_air_time += ctx.dt;
            self.stand_up_timer = 0.0;
            if !game.debug_settings.disable_ragdoll && self.in_air_time >= self.max_in_air_time {
                self.set_ragdoll_enabled(&mut ctx.scene.graph, true);
            }
        }
        let finished = game.level.leaderboard.is_finished(ctx.handle);
        if finished {
            // Stand still.
            self.target_desired_velocity.x = 0.0;
            self.target_desired_velocity.z = 0.0;
        }

        if self.has_serious_impact(ctx) {
            self.in_air_time = 999.0;
        }

        let y_vel = self.target_desired_velocity.y;
        self.desired_velocity.follow(
            &self.target_desired_velocity,
            if has_ground_contact { 0.2 } else { 0.1 },
        );
        self.desired_velocity.y = y_vel;

        self.do_move(self.desired_velocity, &mut ctx.scene.graph);

        if let Some(absm) = ctx
            .scene
            .graph
            .try_get_mut_of_type::<AnimationBlendingStateMachine>(self.absm)
        {
            absm.machine_mut()
                .get_value_mut_silent()
                .set_parameter(
                    "Run",
                    Parameter::Rule(self.desired_velocity.xz().norm() >= 0.75 * self.speed),
                )
                .set_parameter("Jump", Parameter::Rule(self.jump));
        }

        self.process_animation_events(ctx, has_ground_contact);

        self.jump_interval -= ctx.dt;

        self.jump = false;
    }
}
