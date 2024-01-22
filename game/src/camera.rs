//! Camera controller for the main player (host). It smoothly follows the host and has obstacle
//! avoiding functionality.

use crate::{Event, Game};
use fyrox::{
    core::{
        algebra::{Point3, UnitQuaternion, Vector3},
        arrayvec::ArrayVec,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    event::DeviceEvent,
    scene::{collider::Collider, graph::physics::RayCastOptions, node::Node},
    script::{ScriptContext, ScriptTrait},
};
use std::ops::Range;

#[derive(Clone, Visit, Debug, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "0c45d21f-878e-4aa5-b4e1-097aaa44f314")]
#[visit(optional)]
pub struct CameraController {
    #[reflect(description = "Handle of a node that has Player script.")]
    anchor: Handle<Node>,
    #[reflect(description = "Default distance from the hinge to the camera.")]
    default_distance: f32,
    #[reflect(description = "Handle of camera hinge.")]
    hinge: Handle<Node>,
    #[reflect(description = "Handle of Camera node.")]
    camera: Handle<Node>,
    #[reflect(description = "Distance from first blocker that in the way of camera.")]
    probe_radius: f32,
    #[reflect(description = "Pitch range for camera")]
    pitch_range: Range<f32>,
    #[reflect(description = "A collider that should be ignored by ray casting.")]
    pub collider_to_ignore: Handle<Node>,

    #[visit(skip)]
    #[reflect(hidden)]
    target_position: Vector3<f32>,

    #[visit(skip)]
    #[reflect(hidden)]
    pub pitch: f32,

    #[visit(skip)]
    #[reflect(hidden)]
    pub yaw: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            anchor: Default::default(),
            hinge: Default::default(),
            camera: Default::default(),
            pitch: 0.0,
            default_distance: 2.0,
            probe_radius: 0.2,
            pitch_range: -90.0f32..90.0f32,
            yaw: 0.0,
            collider_to_ignore: Default::default(),
            target_position: Default::default(),
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

            // Filter out ragdoll colliders.
            if let Some(collider) = context
                .scene
                .graph
                .try_get_of_type::<Collider>(intersection.collider)
            {
                if collider.collision_groups().memberships.0 & 0b0000_0010 == 0b0000_0010 {
                    continue;
                }
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

impl ScriptTrait for CameraController {
    fn on_os_event(&mut self, event: &Event<()>, ctx: &mut ScriptContext) {
        if let Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } = event
        {
            self.yaw -= delta.0 as f32 * ctx.dt;
            self.pitch = (self.pitch + delta.1 as f32 * ctx.dt).clamp(
                self.pitch_range.start.to_radians(),
                self.pitch_range.end.to_radians(),
            );
        }
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        if game.server.is_none() {
            return;
        }

        if let Some(anchor) = ctx.scene.graph.try_get(self.anchor) {
            self.target_position = anchor.global_position();

            let controller = &mut ctx.scene.graph[ctx.handle];

            let local_transform = controller.local_transform_mut();
            let new_position = **local_transform.position()
                + (self.target_position - **local_transform.position()) * 0.1;
            local_transform.set_rotation(UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                self.yaw,
            ));
            local_transform.set_position(new_position);

            if let Some(hinge) = ctx.scene.graph.try_get_mut(self.hinge) {
                hinge
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::x_axis(),
                        self.pitch,
                    ));

                let hinge_position = hinge.global_position();
                if let Some(camera) = ctx.scene.graph.try_get(self.camera) {
                    self.check_for_obstacles(
                        hinge_position,
                        camera.global_position(),
                        ctx,
                        self.collider_to_ignore,
                    );
                }
            }
        }
    }
}
