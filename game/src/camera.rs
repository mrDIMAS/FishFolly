use crate::{Game, Player};
use fyrox::{
    core::{
        algebra::{Point3, UnitQuaternion, Vector3},
        arrayvec::ArrayVec,
        inspect::prelude::*,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    scene::{
        graph::physics::RayCastOptions,
        node::{Node, TypeUuidProvider},
    },
    script::{ScriptContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Inspect, Visit, Debug)]
pub struct CameraController {
    player: Handle<Node>,
    default_distance: f32,
    hinge: Handle<Node>,
    camera: Handle<Node>,
    probe_radius: f32,

    #[inspect(skip)]
    #[visit(skip)]
    target_position: Vector3<f32>,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            player: Default::default(),
            hinge: Default::default(),
            camera: Default::default(),
            target_position: Default::default(),
            default_distance: 2.0,
            probe_radius: 0.2,
        }
    }
}

impl CameraController {
    fn check_for_obstacles(
        &self,
        begin: Vector3<f32>,
        end: Vector3<f32>,
        context: &mut ScriptContext,
        player_collider: Handle<Node>,
    ) {
        let mut buffer = ArrayVec::<_, 64>::new();

        let dir = (end - begin)
            .try_normalize(f32::EPSILON)
            .unwrap_or_default()
            .scale(self.default_distance);

        context.scene.graph.physics.cast_ray(
            RayCastOptions {
                ray_origin: Point3::from(begin),
                ray_direction: dir,
                max_len: dir.norm(),
                groups: Default::default(),
                sort_results: true,
            },
            &mut buffer,
        );

        let mut distance = self.default_distance;
        for intersection in buffer {
            if intersection.collider == player_collider {
                continue;
            }

            let new_offset = intersection.toi;
            if new_offset < distance {
                distance = new_offset;
            }
        }

        context.scene.graph[self.camera]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 0.0, -distance + self.probe_radius));
    }
}

impl TypeUuidProvider for CameraController {
    fn type_uuid() -> Uuid {
        uuid!("0c45d21f-878e-4aa5-b4e1-097aaa44f314")
    }
}

impl ScriptTrait for CameraController {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args,
            Self::PLAYER => player,
            Self::HINGE => hinge,
            Self::CAMERA => camera,
            Self::PROBE_RADIUS => probe_radius,
            Self::DEFAULT_DISTANCE => default_distance
        )
    }

    fn on_update(&mut self, mut context: ScriptContext) {
        if let Some(player) = context.scene.graph.try_get(self.player) {
            // Sync position with player.
            self.target_position = player.global_position();

            if let Some(player_script) = player.script.as_ref().and_then(|s| s.cast::<Player>()) {
                let yaw = player_script.input_controller.yaw;
                let pitch = player_script.input_controller.pitch;
                let player_collider = player_script.collider;

                let controller = &mut context.scene.graph[context.handle];

                let local_transform = controller.local_transform_mut();
                let new_position = **local_transform.position()
                    + (self.target_position - **local_transform.position()) * 0.1;
                local_transform
                    .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw));
                local_transform.set_position(new_position);

                if let Some(hinge) = context.scene.graph.try_get_mut(self.hinge) {
                    hinge
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch));

                    let hinge_position = hinge.global_position();
                    if let Some(camera) = context.scene.graph.try_get(self.camera) {
                        self.check_for_obstacles(
                            hinge_position,
                            camera.global_position(),
                            &mut context,
                            player_collider,
                        );
                    }
                }
            } else {
                Log::warn("Must be player script!".to_owned())
            }
        } else {
            Log::warn("Player is not set!".to_owned());
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
