use crate::{Game, Player};
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        inspect::prelude::*,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    scene::{node::Node, node::TypeUuidProvider},
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Default, Inspect, Visit, Debug)]
pub struct CameraController {
    player: Handle<Node>,

    #[visit(optional)]
    hinge: Handle<Node>,

    #[inspect(skip)]
    #[visit(skip)]
    target_position: Vector3<f32>,
}

impl TypeUuidProvider for CameraController {
    fn type_uuid() -> Uuid {
        uuid!("0c45d21f-878e-4aa5-b4e1-097aaa44f314")
    }
}

impl ScriptTrait for CameraController {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args, Self::PLAYER => player, Self::HINGE => hinge)
    }

    fn on_update(&mut self, context: ScriptContext) {
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        if let Some(player) = context.scene.graph.try_get(self.player) {
            // Sync position with player.
            self.target_position = player.global_position();

            if let Some(player_script) = player.script.as_ref().and_then(|s| s.cast::<Player>()) {
                yaw = player_script.input_controller.yaw;
                pitch = player_script.input_controller.pitch;
            }
        }

        let camera = &mut context.scene.graph[context.handle];

        let local_transform = camera.local_transform_mut();
        let new_position = **local_transform.position()
            + (self.target_position - **local_transform.position()) * 0.1;
        local_transform.set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw));
        local_transform.set_position(new_position);

        if let Some(hinge) = context.scene.graph.try_get_mut(self.hinge) {
            hinge
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch));
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
