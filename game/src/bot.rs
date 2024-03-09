//! A simple bot that tries to react Target points on a level.

use crate::{actor::Actor, actor::ActorMessage, utils, Game};
use fyrox::{
    core::{
        algebra::{Point3, UnitQuaternion, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        log::Log,
        parking_lot::Mutex,
        parking_lot::RwLock,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    graph::{BaseSceneGraph, SceneGraph},
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

#[derive(Clone, Debug, Default)]
struct DebugData {
    lines: Vec<Line>,
}

#[derive(Debug, Default)]
struct DebugDataWrapper(Mutex<DebugData>);

impl DebugDataWrapper {
    fn add_line(&self, begin: Vector3<f32>, end: Vector3<f32>, color: Color) {
        self.0.lock().lines.push(Line { begin, end, color })
    }
}

impl Clone for DebugDataWrapper {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().clone()))
    }
}

#[derive(Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "85980387-81c0-4115-a74b-f9875084f464")]
#[visit(optional)]
pub struct Bot {
    #[reflect(description = "Handle of an edge probe locator begin node")]
    probe_begin: Handle<Node>,
    #[reflect(description = "Handle of an edge probe locator end node")]
    probe_end: Handle<Node>,
    #[component(include)]
    pub actor: Actor,
    #[visit(skip)]
    #[reflect(hidden)]
    agent: NavmeshAgent,
    #[visit(skip)]
    #[reflect(hidden)]
    navmesh: Option<Arc<RwLock<Navmesh>>>,
    #[visit(skip)]
    #[reflect(hidden)]
    debug_data: DebugDataWrapper,
}

impl Default for Bot {
    fn default() -> Self {
        Self {
            actor: Default::default(),
            probe_begin: Default::default(),
            probe_end: Default::default(),
            agent: NavmeshAgentBuilder::new()
                .with_recalculation_threshold(2.0)
                .build(),
            navmesh: Default::default(),
            debug_data: Default::default(),
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

fn height_difference<F>(
    begin: Vector3<f32>,
    max_height: f32,
    graph: &Graph,
    debug: F,
) -> Option<f32>
where
    F: FnOnce(Vector3<f32>),
{
    match probe_ground(begin, max_height, graph) {
        Some(pos) => {
            debug(pos);
            Some(pos.metric_distance(&begin))
        }
        None => {
            debug(begin + Vector3::new(0.0, -max_height, 0.0));
            None
        }
    }
}

fn is_safe_height_difference<F>(
    begin: Vector3<f32>,
    max_height: f32,
    graph: &Graph,
    debug: F,
) -> bool
where
    F: FnOnce(Vector3<f32>),
{
    height_difference(begin, max_height, graph, debug).map_or(false, |diff| diff <= 8.0)
}

impl Bot {
    fn debug_draw(&self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
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

            let debug_data = self.debug_data.0.lock();
            for line in debug_data.lines.iter() {
                ctx.scene.drawing_context.add_line(line.clone());
                ctx.scene
                    .drawing_context
                    .draw_sphere(line.begin, 16, 16, 0.25, line.color);

                ctx.scene
                    .drawing_context
                    .draw_sphere(line.end, 16, 16, 0.25, line.color);
            }
        }
    }

    // Checks if there are a gap on the way, that can be jumped over.
    fn need_jump_over(&self, ctx: &ScriptContext) -> bool {
        let graph = &ctx.scene.graph;

        let Some(begin) = graph.try_get(self.probe_begin).map(|n| n.global_position()) else {
            return false;
        };

        let Some(end) = graph.try_get(self.probe_end).map(|n| n.global_position()) else {
            return false;
        };

        let middle = (begin + end).scale(0.5);

        let max_height = 10.0;

        // Bot can just jump down and it will be fine.
        if is_safe_height_difference(begin, max_height, graph, |p| {
            self.debug_data.add_line(begin, p, Color::YELLOW);
        }) {
            return false;
        }

        // Otherwise there might be a gap between the two probe points.
        if is_safe_height_difference(middle, max_height, graph, |p| {
            self.debug_data.add_line(middle, p, Color::PINK);
        }) {
            return false;
        }

        // If a bot have a platform to jump on right in front of it, then it needs to jump.
        is_safe_height_difference(end, max_height, graph, |p| {
            self.debug_data.add_line(end, p, Color::ORANGE);
        })
    }
}

impl ScriptTrait for Bot {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        ctx.plugins
            .get_mut::<Game>()
            .level
            .actors
            .insert(ctx.handle);
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
        ctx.plugins
            .get_mut::<Game>()
            .level
            .actors
            .remove(&ctx.node_handle);
        Log::info(format!("Bot {:?} destroyed!", ctx.node_handle));
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        self.debug_data.0.lock().lines.clear();

        let game = ctx.plugins.get::<Game>();
        if game.is_client() {
            return;
        }

        let is_in_jump_state = self.actor.is_in_jump_state(&ctx.scene.graph);

        // Dead-simple AI - run straight to target.
        let target_pos = game
            .level
            .targets
            .iter()
            .next()
            .cloned()
            .map(|t| ctx.scene.graph[t].global_position());

        let need_jump_over = self.need_jump_over(ctx);
        let has_ground_contact = utils::has_ground_contact(self.actor.collider, &ctx.scene.graph);

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

                let jump_y_vel =
                    if has_ground_contact && !is_in_jump_state && self.actor.jump_interval <= 0.0 {
                        if need_jump_over {
                            self.actor.jump();
                            self.actor.jump_vel
                        } else {
                            0.0
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

        self.debug_draw(ctx);
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
