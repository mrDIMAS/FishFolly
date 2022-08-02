//! A simple bot that tries to react Target points on a level.

use crate::{game_mut, marker::Actor, utils, Game, GameConstructor, Ragdoll};
use fyrox::{
    animation::machine::{Machine, Parameter},
    core::{
        algebra::Point3, algebra::UnitQuaternion, algebra::Vector3, arrayvec::ArrayVec,
        futures::executor::block_on, inspect::prelude::*, pool::Handle, reflect::Reflect,
        uuid::uuid, uuid::Uuid, visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_component_provider,
    resource::absm::AbsmResource,
    scene::{
        collider::{Collider, ColliderShape},
        graph::{map::NodeHandleMap, physics::RayCastOptions, Graph},
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
    },
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::{
        log::Log,
        navmesh::{NavmeshAgent, NavmeshAgentBuilder},
    },
};

#[derive(Clone, Visit, Inspect, Reflect, Debug)]
pub struct Bot {
    #[inspect(description = "Speed of the bot.")]
    speed: f32,
    #[inspect(description = "Handle of a model of the bot.")]
    model_root: Handle<Node>,
    #[inspect(description = "Animation blending state machine used by bot's model.")]
    absm_resource: Option<AbsmResource>,
    #[inspect(description = "Collider of the bot.")]
    pub collider: Handle<Node>,
    #[inspect(description = "Handle of an edge probe locator node")]
    probe_locator: Handle<Node>,
    #[inspect(description = "A handle of the ragdoll")]
    #[visit(optional)]
    ragdoll: Handle<Node>,
    #[inspect(
        description = "Amount of time that the bot will be lying on the ground with active ragdoll."
    )]
    #[visit(optional)]
    stand_up_timeout: f32,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    absm: Handle<Machine>,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    pub actor: Actor,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    agent: NavmeshAgent,
    #[visit(skip)]
    #[inspect(skip)]
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
            model_root: Default::default(),
            absm_resource: None,
            collider: Default::default(),
            absm: Default::default(),
            actor: Default::default(),
            probe_locator: Default::default(),
            agent: NavmeshAgentBuilder::new()
                .with_recalculation_threshold(0.5)
                .build(),
            ragdoll: Default::default(),
            stand_up_timeout: 2.0,
            stand_up_timer: 0.0,
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
    fn on_init(&mut self, context: ScriptContext) {
        assert!(game_mut(context.plugin).actors.insert(context.handle));

        if context.scene.graph.is_valid_handle(self.model_root) {
            if let Some(absm) = self.absm_resource.as_ref() {
                let animations = block_on(absm.load_animations(context.resource_manager.clone()));

                self.absm = absm
                    .instantiate(self.model_root, context.scene, animations)
                    .unwrap();
            }
        }
        Log::info(format!("Bot {:?} created!", context.handle));
    }

    fn on_deinit(&mut self, context: ScriptDeinitContext) {
        assert!(game_mut(context.plugin).actors.remove(&context.node_handle));
        Log::info(format!("Bot {:?} destroyed!", context.node_handle));
    }

    fn on_update(&mut self, context: ScriptContext) {
        let ScriptContext {
            scene,
            handle,
            plugin,
            dt,
            ..
        } = context;

        let plugin = plugin.cast::<Game>().unwrap();

        // Dead-simple AI - run straight to target.
        let target_pos = plugin
            .targets
            .iter()
            .next()
            .cloned()
            .map(|t| scene.graph[t].global_position());

        let ground_probe_begin =
            if let Some(probe_locator) = scene.graph.try_get(self.probe_locator) {
                probe_locator.global_position()
            } else {
                Log::warn("There is not ground probe locator specified!".to_owned());
                Default::default()
            };

        if let Some(target_pos) = target_pos {
            if let Some(rigid_body) = scene.graph[handle].cast_mut::<RigidBody>() {
                let self_position = rigid_body.global_position();
                let current_y_lin_vel = rigid_body.lin_vel().y;

                if let Some(navmesh) = scene.navmeshes.at_mut(0) {
                    self.agent.set_speed(self.speed);
                    self.agent.set_target(target_pos);
                    self.agent.set_position(self_position);
                    let _ = self.agent.update(context.dt, navmesh);
                }

                let has_reached_destination =
                    self.agent.target().metric_distance(&self_position) <= 1.0;
                let horizontal_velocity = if has_reached_destination {
                    Vector3::new(0.0, 0.0, 0.0)
                } else {
                    let mut vel = (self.agent.position() - self_position).scale(1.0 / context.dt);
                    vel.y = 0.0;
                    vel
                };

                let mut jump = false;
                let jump_vel = 5.0;
                let y_vel = if utils::has_ground_contact(self.collider, &scene.graph) {
                    if let Some(probed_position) =
                        probe_ground(ground_probe_begin, 10.0, &scene.graph)
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
                self.stand_up_timer -= dt;
                if jump && self.stand_up_timer <= 0.0 {
                    set_ragdoll_enabled(self.ragdoll, &mut scene.graph, true);
                    self.stand_up_timer = self.stand_up_timeout;
                }
                if self.stand_up_timer <= 0.0 {
                    set_ragdoll_enabled(self.ragdoll, &mut scene.graph, false);
                }

                // Reborrow the node.
                let rigid_body = scene.graph[handle].cast_mut::<RigidBody>().unwrap();
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

                if let Some(absm) = scene.animation_machines.try_get_mut(self.absm) {
                    absm.set_parameter("Run", Parameter::Rule(is_running))
                        .set_parameter("Jump", Parameter::Rule(jump));
                }
            }
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping
            .map(&mut self.model_root)
            .map(&mut self.collider)
            .map(&mut self.probe_locator)
            .map(&mut self.ragdoll);
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        let mut state = resource_manager.state();
        let containers = state.containers_mut();
        containers
            .absm
            .try_restore_optional_resource(&mut self.absm_resource);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}
