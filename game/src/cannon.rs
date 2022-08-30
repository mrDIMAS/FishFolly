//! Cannon shoots large balls that push players (or bots) off the platforms.

use crate::GameConstructor;
use fyrox::{
    core::{
        inspect::prelude::*,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_component_provider, impl_directly_inheritable_entity_trait,
    resource::model::Model,
    scene::{node::TypeUuidProvider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Debug, Visit, Inspect, Reflect)]
pub struct Cannon {
    ball_prefab: Option<Model>,
    shooting_timeout: f32,
    #[visit(optional)]
    shooting_force: f32,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    timer: f32,
}

impl_component_provider!(Cannon);
impl_directly_inheritable_entity_trait!(Cannon;);

impl Default for Cannon {
    fn default() -> Self {
        Self {
            ball_prefab: None,
            shooting_timeout: 2.0,
            timer: 0.0,
            shooting_force: 100.0,
        }
    }
}

impl TypeUuidProvider for Cannon {
    fn type_uuid() -> Uuid {
        uuid!("becf5c5f-c745-40ee-85c9-491656fd222e")
    }
}

impl ScriptTrait for Cannon {
    fn on_update(&mut self, context: ScriptContext) {
        self.timer += context.dt;
        if self.timer >= self.shooting_timeout {
            self.timer = 0.0;

            let self_node = &context.scene.graph[context.handle];
            let self_position = self_node.global_position();
            let shooting_dir = self_node
                .look_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default();
            if let Some(ball_prefab) = self.ball_prefab.as_ref() {
                let ball_instance = ball_prefab.instantiate_geometry(context.scene);
                context.scene.graph[ball_instance].set_lifetime(Some(5.0));

                let body = context
                    .scene
                    .graph
                    .find(ball_instance, &mut |node| node.tag() == "Body");
                if let Some(body) = context.scene.graph.try_get_mut(body) {
                    body.local_transform_mut().set_position(self_position);

                    if let Some(rigid_body) = body.cast_mut::<RigidBody>() {
                        rigid_body.set_lin_vel(shooting_dir.scale(self.shooting_force));
                    }
                } else {
                    Log::warn("Cannot find Body of ball!".to_owned());
                }
            }
        }
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        resource_manager
            .state()
            .containers_mut()
            .models
            .try_restore_optional_resource(&mut self.ball_prefab);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}
