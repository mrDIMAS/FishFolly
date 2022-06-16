use crate::{Game, Uuid};
use fyrox::{
    core::{algebra::Vector3, inspect::prelude::*, uuid::uuid, visitor::prelude::*},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    scene::{node::TypeUuidProvider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Visit, Inspect, Debug)]
pub struct Bot {
    #[visit(optional)]
    speed: f32,
}

impl TypeUuidProvider for Bot {
    fn type_uuid() -> Uuid {
        uuid!("85980387-81c0-4115-a74b-f9875084f464")
    }
}

impl Default for Bot {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

impl ScriptTrait for Bot {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args, Self::SPEED => speed)
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

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
