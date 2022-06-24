//! A simple bot that tries to react Target points on a level.

use crate::{game_mut, marker::Actor, Game};
use fyrox::{
    animation::machine::{Machine, Parameter},
    core::{
        algebra::UnitQuaternion, algebra::Vector3, futures::executor::block_on,
        inspect::prelude::*, pool::Handle, uuid::uuid, uuid::Uuid, visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    resource::absm::AbsmResource,
    scene::{
        graph::map::NodeHandleMap,
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
    },
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Visit, Inspect, Debug)]
pub struct Bot {
    #[inspect(description = "Speed of the bot.")]
    speed: f32,
    #[inspect(description = "Handle of a model of the bot.")]
    model_root: Handle<Node>,
    #[inspect(description = "Animation blending state machine used by bot's model.")]
    absm_resource: Option<AbsmResource>,
    #[visit(skip)]
    #[inspect(skip)]
    absm: Handle<Machine>,
    #[visit(skip)]
    #[inspect(skip)]
    pub actor: Actor,
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
            absm: Default::default(),
            actor: Default::default(),
        }
    }
}

impl ScriptTrait for Bot {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args,
            Self::SPEED => speed,
            Self::ABSM_RESOURCE => absm_resource,
            Self::MODEL_ROOT => model_root)
    }

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

        if let Some(target_pos) = target_pos {
            if let Some(rigid_body) = scene.graph[handle].cast_mut::<RigidBody>() {
                let target_vec = target_pos - rigid_body.global_position();
                let distance = target_vec.norm();
                let dir = target_vec.try_normalize(f32::EPSILON).unwrap_or_default();

                let reached_target = distance < 2.5;
                let horizontal_velocity = if reached_target {
                    Vector3::default()
                } else {
                    Vector3::new(dir.x * self.speed, 0.0, dir.z * self.speed)
                };

                rigid_body.set_lin_vel(Vector3::new(
                    horizontal_velocity.x,
                    rigid_body.lin_vel().y,
                    horizontal_velocity.z,
                ));

                let is_running = horizontal_velocity.norm() > 0.1;

                if is_running {
                    rigid_body
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::face_towards(
                            &horizontal_velocity,
                            &Vector3::y_axis(),
                        ));
                }

                if let Some(absm) = scene.animation_machines.try_get_mut(self.absm) {
                    absm.set_parameter("Run", Parameter::Rule(is_running));
                }
            }
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping.map(&mut self.model_root);
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
        Game::type_uuid()
    }
}
