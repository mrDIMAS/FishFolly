//! A dynamic (rotating, moving) obstacle.

use crate::Game;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, UnitVector3, Vector3},
        inspect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    scene::{node::TypeUuidProvider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
};

/// TODO: Ideally any animation for obstacles should be done in the editor, but there is no
/// animation editor yet.
#[derive(Clone, Debug, Visit, Inspect)]
pub struct RotatorObstacle {
    angle: f32,
    axis: Vector3<f32>,
    speed: f32,
}

impl_component_provider!(RotatorObstacle);

impl Default for RotatorObstacle {
    fn default() -> Self {
        Self {
            angle: 0.0,
            axis: Default::default(),
            speed: 2.0,
        }
    }
}

impl TypeUuidProvider for RotatorObstacle {
    fn type_uuid() -> Uuid {
        uuid!("54ce703d-a56c-4534-a8a8-33ee1c6dd0a2")
    }
}

impl ScriptTrait for RotatorObstacle {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args,
            Self::ANGLE => angle,
            Self::AXIS => axis,
            Self::SPEED => speed
        )
    }

    fn on_update(&mut self, context: ScriptContext) {
        self.angle += self.speed * context.dt;

        if let Some(rigid_body) = context.scene.graph[context.handle].cast_mut::<RigidBody>() {
            rigid_body
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &UnitVector3::new_normalize(self.axis),
                    self.angle,
                ));
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
