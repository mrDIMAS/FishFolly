//! A simple bot that tries to react Target points on a level.

use crate::{game_mut, game_ref, marker::Actor, utils, Ragdoll};
use fyrox::{
    animation::machine::Parameter,
    core::{
        algebra::Point3, algebra::UnitQuaternion, algebra::Vector3, arrayvec::ArrayVec,
        pool::Handle, reflect::prelude::*, uuid::uuid, uuid::Uuid, visitor::prelude::*,
    },
    impl_component_provider,
    scene::{
        animation::absm::AnimationBlendingStateMachine,
        collider::{Collider, ColliderShape},
        graph::{physics::RayCastOptions, Graph},
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
    },
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::{
        log::Log,
        navmesh::{NavmeshAgent, NavmeshAgentBuilder},
    },
};

#[derive(Clone, Visit, Reflect, Debug)]
pub struct Bot {
    #[reflect(description = "Speed of the bot.")]
    speed: f32,
    #[reflect(description = "Collider of the bot.")]
    pub collider: Handle<Node>,
    #[reflect(description = "Handle of an edge probe locator node")]
    probe_locator: Handle<Node>,
    #[reflect(description = "Handle of animation state machine.")]
    absm: Handle<Node>,
    #[reflect(description = "A handle of the ragdoll")]
    #[visit(optional)]
    ragdoll: Handle<Node>,
    #[reflect(
        description = "Amount of time that the bot will be lying on the ground with active ragdoll."
    )]
    #[visit(optional)]
    stand_up_timeout: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    pub actor: Actor,
    #[visit(skip)]
    #[reflect(hidden)]
    agent: NavmeshAgent,
    #[visit(skip)]
    #[reflect(hidden)]
    stand_up_timer: f32,
}

impl_component_provider!(Bot, actor: Actor);

impl TypeUuidProvider for Bot {
    fn type_uuid() -> Uuid {
        uuid!("85980387-81c0-4115-a74b-f9875084f464")
    }
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            speed: 1.0,
            collider: Default::default(),
            actor: Default::default(),
            probe_locator: Default::default(),
            agent: NavmeshAgentBuilder::new()
                .with_recalculation_threshold(0.5)
                .build(),
            ragdoll: Default::default(),
            stand_up_timeout: 2.0,
            stand_up_timer: 0.0,
            absm: Default::default(),
        }
    }
}

fn probe_ground(begin: Vector3<f32>, max_height: f32, graph: &Graph) -> Option<Vector3<f32>> {
    let mut buffer = ArrayVec::<_, 64>::new();

    let end = Vector3::new(begin.x, begin.y - max_height, begin.z);

    let dir = (end - begin)
        .try_normalize(f32::EPSILON)
        .unwrap_or_default()
        .scale(max_height);

    graph.physics.cast_ray(
        RayCastOptions {
            ray_origin: Point3::from(begin),
            ray_direction: dir,
            max_len: dir.norm(),
            groups: Default::default(),
            sort_results: true,
        },
        &mut buffer,
    );

    for intersection in buffer {
        if let Some(collider) = graph[intersection.collider].cast::<Collider>() {
            if let ColliderShape::Trimesh(_) = collider.shape() {
                return Some(intersection.position.coords);
            }
        }
    }

    None
}

fn set_ragdoll_enabled(ragdoll_holder: Handle<Node>, graph: &mut Graph, enabled: bool) {
    if let Some(ragdoll) = graph
        .try_get_mut(ragdoll_holder)
        .and_then(|n| n.script_mut())
        .and_then(|s| s.cast_mut::<Ragdoll>())
    {
        ragdoll.enabled = enabled;
    }
}

impl ScriptTrait for Bot {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(game_mut(ctx.plugins).actors.insert(ctx.handle));
        Log::info(format!("Bot {:?} created!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(game_mut(ctx.plugins).actors.remove(&ctx.node_handle));
        Log::info(format!("Bot {:?} destroyed!", ctx.node_handle));
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = game_ref(ctx.plugins);

        // Dead-simple AI - run straight to target.
        let target_pos = game
            .targets
            .iter()
            .next()
            .cloned()
            .map(|t| ctx.scene.graph[t].global_position());

        let ground_probe_begin =
            if let Some(probe_locator) = ctx.scene.graph.try_get(self.probe_locator) {
                probe_locator.global_position()
            } else {
                Log::warn("There is not ground probe locator specified!");
                Default::default()
            };

        if let Some(target_pos) = target_pos {
            if let Some(rigid_body) = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>() {
                let self_position = rigid_body.global_position();
                let current_y_lin_vel = rigid_body.lin_vel().y;

                if let Some(navmesh) = ctx.scene.navmeshes.at_mut(0) {
                    self.agent.set_speed(self.speed);
                    self.agent.set_target(target_pos);
                    self.agent.set_position(self_position);
                    let _ = self.agent.update(ctx.dt, navmesh);
                }

                let has_reached_destination =
                    self.agent.target().metric_distance(&self_position) <= 1.0;
                let horizontal_velocity = if has_reached_destination {
                    Vector3::new(0.0, 0.0, 0.0)
                } else {
                    let mut vel = (self.agent.position() - self_position).scale(1.0 / ctx.dt);
                    vel.y = 0.0;
                    vel
                };

                let mut jump = false;
                let jump_vel = 5.0;
                let y_vel = if utils::has_ground_contact(self.collider, &ctx.scene.graph) {
                    if let Some(probed_position) =
                        probe_ground(ground_probe_begin, 10.0, &ctx.scene.graph)
                    {
                        if probed_position.metric_distance(&ground_probe_begin) > 8.0 {
                            jump = true;
                            jump_vel
                        } else {
                            current_y_lin_vel
                        }
                    } else {
                        jump = true;
                        jump_vel
                    }
                } else {
                    current_y_lin_vel
                };

                // TEST - activate ragdoll on jumping
                self.stand_up_timer -= ctx.dt;
                if jump && self.stand_up_timer <= 0.0 {
                    set_ragdoll_enabled(self.ragdoll, &mut ctx.scene.graph, true);
                    self.stand_up_timer = self.stand_up_timeout;
                }
                if self.stand_up_timer <= 0.0 {
                    set_ragdoll_enabled(self.ragdoll, &mut ctx.scene.graph, false);
                }

                // Reborrow the node.
                let rigid_body = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>().unwrap();
                rigid_body.set_lin_vel(Vector3::new(
                    horizontal_velocity.x,
                    y_vel,
                    horizontal_velocity.z,
                ));

                let is_running = self.stand_up_timer <= 0.0 && horizontal_velocity.norm() > 0.1;

                if is_running {
                    rigid_body
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::face_towards(
                            &horizontal_velocity,
                            &Vector3::y_axis(),
                        ));
                }

                if let Some(absm) = ctx
                    .scene
                    .graph
                    .try_get_mut(self.absm)
                    .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
                {
                    absm.machine_mut()
                        .get_value_mut_silent()
                        .set_parameter("Run", Parameter::Rule(is_running))
                        .set_parameter("Jump", Parameter::Rule(jump));
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
