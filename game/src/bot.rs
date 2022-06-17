use crate::Game;
use fyrox::{
    animation::machine::Machine,
    core::{
        algebra::Vector3, futures::executor::block_on, inspect::prelude::*, pool::Handle,
        uuid::uuid, uuid::Uuid, visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    fxhash::FxHashMap,
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    resource::absm::AbsmResource,
    scene::{
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
    },
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Visit, Inspect, Debug)]
pub struct Bot {
    #[visit(optional)]
    speed: f32,
    #[visit(optional)]
    model_root: Handle<Node>,
    #[visit(optional)]
    absm_resource: Option<AbsmResource>,

    #[visit(skip)]
    #[inspect(skip)]
    absm: Handle<Machine>,
}

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
        if context.scene.graph.is_valid_handle(self.model_root) {
            if let Some(absm) = self.absm_resource.as_ref() {
                let animations = block_on(absm.load_animations(context.resource_manager.clone()));

                self.absm = absm
                    .instantiate(self.model_root, context.scene, animations)
                    .unwrap();
            }
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.model_root = old_new_mapping
            .get(&self.model_root)
            .cloned()
            .unwrap_or_default();
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
            .first()
            .cloned()
            .map(|t| scene.graph[t].global_position());

        if let Some(target_pos) = target_pos {
            if let Some(rigid_body) = scene.graph[handle].cast_mut::<RigidBody>() {
                let dir = (target_pos - rigid_body.global_position())
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default();

                let velocity = Vector3::new(
                    dir.x * self.speed,
                    rigid_body.lin_vel().y,
                    dir.z * self.speed,
                );

                rigid_body.set_lin_vel(velocity);
            }
        }
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
