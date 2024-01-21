//! A simple bot that tries to react Target points on a level.

use crate::{actor::Actor, actor::ActorMessage, utils, Game};
use fyrox::{
    core::{
        algebra::{Point3, UnitQuaternion, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        log::Log,
        parking_lot::RwLock,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    scene::{
        collider::{Collider, ColliderShape},
        debug::Line,
        graph::{physics::RayCastOptions, Graph},
        navmesh::NavigationalMesh,
        node::Node,
        rigidbody::RigidBody,
    },
    script::{
        ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload, ScriptTrait,
    },
    utils::navmesh::{Navmesh, NavmeshAgent, NavmeshAgentBuilder},
};
use std::sync::Arc;

#[derive(Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "85980387-81c0-4115-a74b-f9875084f464")]
#[visit(optional)]
pub struct Bot {
    #[reflect(description = "Handle of an edge probe locator node")]
    probe_locator: Handle<Node>,
    #[component(include)]
    pub actor: Actor,
    #[visit(skip)]
    #[reflect(hidden)]
    agent: NavmeshAgent,
    #[visit(skip)]
    #[reflect(hidden)]
    navmesh: Option<Arc<RwLock<Navmesh>>>,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            actor: Default::default(),
            probe_locator: Default::default(),
            agent: NavmeshAgentBuilder::new()
                .with_recalculation_threshold(2.0)
                .build(),
            navmesh: Default::default(),
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

impl ScriptTrait for Bot {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(ctx.plugins.get_mut::<Game>().actors.insert(ctx.handle));
        Log::info(format!("Bot {:?} created!", ctx.handle));
        self.navmesh = ctx
            .scene
            .graph
            .find_from_root(&mut |n| n.is_navigational_mesh())
            .and_then(|(_, n)| n.cast::<NavigationalMesh>())
            .map(|n| n.navmesh());
    }

    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.message_dispatcher
            .subscribe_to::<ActorMessage>(ctx.handle);

        self.agent
            .set_position(ctx.scene.graph[ctx.handle].global_position());
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .actors
            .remove(&ctx.node_handle));
        Log::info(format!("Bot {:?} destroyed!", ctx.node_handle));
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        if game.server.is_none() {
            return;
        }

        let is_in_jump_state = self.actor.is_in_jump_state(&ctx.scene.graph);

        // Dead-simple AI - run straight to target.
        let target_pos = game
            .targets
            .iter()
            .next()
            .cloned()
            .map(|t| ctx.scene.graph[t].global_position());

        if game.debug_settings.show_paths {
            for pts in self.agent.path().windows(2) {
                let a = pts[0];
                let b = pts[1];
                ctx.scene.drawing_context.add_line(Line {
                    begin: a,
                    end: b,
                    color: Color::RED,
                })
            }

            ctx.scene
                .drawing_context
                .draw_sphere(self.agent.target(), 16, 16, 0.25, Color::GREEN);

            ctx.scene.drawing_context.draw_sphere(
                self.agent.position(),
                16,
                16,
                0.25,
                Color::GREEN,
            );

            if let Some(navmesh) = self.navmesh.as_ref() {
                let navmesh = navmesh.read();
                if let Some(closest) =
                    navmesh.query_closest(ctx.scene.graph[self.actor.rigid_body].global_position())
                {
                    ctx.scene
                        .drawing_context
                        .draw_sphere(closest.0, 16, 16, 0.25, Color::BLUE);
                }
            }
        }

        let ground_probe_begin =
            if let Some(probe_locator) = ctx.scene.graph.try_get(self.probe_locator) {
                probe_locator.global_position()
            } else {
                Log::warn("There is not ground probe locator specified!");
                Default::default()
            };

        self.actor.target_desired_velocity = Vector3::new(0.0, 0.0, 0.0);

        if let Some(target_pos) = target_pos {
            if let Some(rigid_body) = ctx.scene.graph[self.actor.rigid_body].cast_mut::<RigidBody>()
            {
                let self_position = rigid_body.global_position();

                if let Some(navmesh) = self.navmesh.as_ref() {
                    let navmesh = navmesh.read();
                    self.agent.set_speed(self.actor.speed);
                    self.agent.set_target(target_pos);
                    self.agent.set_position(self_position);
                    let _ = self.agent.update(ctx.dt, &navmesh);
                }

                let has_reached_destination =
                    self.agent.target().metric_distance(&self_position) <= 1.0;
                let horizontal_velocity = if has_reached_destination {
                    Vector3::new(0.0, 0.0, 0.0)
                } else {
                    let mut vel = (self.agent.position() - self_position)
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_default()
                        .scale(self.actor.speed);
                    vel.y = 0.0;
                    vel
                };

                let jump_y_vel = if utils::has_ground_contact(self.actor.collider, &ctx.scene.graph)
                    && !is_in_jump_state
                    && self.actor.jump_interval <= 0.0
                {
                    if let Some(probed_position) =
                        probe_ground(ground_probe_begin, 10.0, &ctx.scene.graph)
                    {
                        if probed_position.metric_distance(&ground_probe_begin) > 8.0 {
                            self.actor.jump();
                            self.actor.jump_vel
                        } else {
                            0.0
                        }
                    } else {
                        self.actor.jump();
                        self.actor.jump_vel
                    }
                } else {
                    0.0
                };

                self.actor.target_desired_velocity =
                    Vector3::new(horizontal_velocity.x, jump_y_vel, horizontal_velocity.z);

                // Reborrow the node.
                let rigid_body = ctx.scene.graph[self.actor.rigid_body]
                    .cast_mut::<RigidBody>()
                    .unwrap();

                let is_running = horizontal_velocity.norm() > 0.1;

                if is_running {
                    rigid_body
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::face_towards(
                            &horizontal_velocity,
                            &Vector3::y_axis(),
                        ));
                }
            }
        }

        self.actor.on_update(ctx);
    }

    fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) {
        self.actor.on_message(message, ctx);

        let Some(message) = message.downcast_ref::<ActorMessage>() else {
            return;
        };

        match message {
            ActorMessage::RespawnAt(position) => {
                self.agent.set_position(*position);
            }
        }
    }
}
