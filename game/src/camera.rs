//! Camera controller for the main player (host). It smoothly follows the host and has obstacle
//! avoiding functionality.

use crate::Game;
use fyrox::graph::SceneGraph;
use fyrox::plugin::error::GameResult;
use fyrox::scene::camera::Camera;
use fyrox::{
    core::{
        algebra::{Point3, Vector3},
        arrayvec::ArrayVec,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    scene::{collider::Collider, graph::physics::RayCastOptions, node::Node},
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Visit, Debug, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "0c45d21f-878e-4aa5-b4e1-097aaa44f314")]
#[visit(optional)]
pub struct CameraController {
    /// Default distance from the hinge to the camera.
    default_distance: f32,
    /// Handle of camera hinge.
    hinge: Handle<Node>,
    /// Handle of Camera node.
    pub camera: Handle<Camera>,
    /// Distance from first blocker that in the way of camera.
    probe_radius: f32,
    /// A collider that should be ignored by ray casting.
    pub collider_to_ignore: Handle<Collider>,
    #[reflect(hidden)]
    pub target_position: Vector3<f32>,
    #[reflect(hidden)]
    pub pitch: f32,
    #[reflect(hidden)]
    pub yaw: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            hinge: Default::default(),
            camera: Default::default(),
            pitch: 0.0,
            default_distance: 2.0,
            probe_radius: 0.2,
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
        player_collider: Handle<Collider>,
    ) -> GameResult {
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
            let collider = context.scene.graph.try_get(intersection.collider)?;
            if collider.collision_groups().memberships.0 & 0b0000_0010 == 0b0000_0010 {
                continue;
            }

            let new_offset = intersection.toi;
            if new_offset < distance {
                distance = new_offset;
            }
        }

        let camera = context.scene.graph.try_get_mut(self.camera)?;
        camera.set_position_xyz(0.0, 0.0, -distance + self.probe_radius);
        Ok(())
    }
}

impl ScriptTrait for CameraController {
    fn on_update(&mut self, ctx: &mut ScriptContext) -> GameResult {
        if ctx.plugins.get::<Game>().is_client() {
            return Ok(());
        }

        let controller = &mut ctx.scene.graph[ctx.handle];

        let local_transform = controller.local_transform_mut();
        let new_position = **local_transform.position()
            + (self.target_position - **local_transform.position()) * 0.1;
        local_transform.set_position(new_position);
        controller.set_rotation_y(self.yaw);

        let hinge = ctx.scene.graph.try_get_mut(self.hinge)?;
        hinge.set_rotation_x(self.pitch);
        let hinge_position = hinge.global_position();
        let camera = ctx.scene.graph.try_get(self.camera)?;
        self.check_for_obstacles(
            hinge_position,
            camera.global_position(),
            ctx,
            self.collider_to_ignore,
        )
    }
}
