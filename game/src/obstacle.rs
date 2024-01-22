//! A dynamic (rotating, moving) obstacle.

use crate::Game;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, UnitVector3, Vector3},
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    scene::rigidbody::RigidBody,
    script::{ScriptContext, ScriptTrait},
};

/// TODO: Ideally any animation for obstacles should be done in the editor, but there is no
/// animation editor yet.
#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "54ce703d-a56c-4534-a8a8-33ee1c6dd0a2")]
pub struct RotatorObstacle {
    angle: f32,
    axis: Vector3<f32>,
    speed: f32,
}

impl Default for RotatorObstacle {
    fn default() -> Self {
        Self {
            angle: 0.0,
            axis: Default::default(),
            speed: 2.0,
        }
    }
}

impl ScriptTrait for RotatorObstacle {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        if game.server.is_none() {
            return;
        }

        self.angle += self.speed * ctx.dt;

        if let Some(rigid_body) = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>() {
            rigid_body
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &UnitVector3::new_normalize(self.axis),
                    self.angle,
                ));
        }
    }
}
