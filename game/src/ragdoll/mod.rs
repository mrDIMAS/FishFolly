//! A ragdoll that is used for characters that fall.
use crate::ragdoll::link::BoneLink;
use fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        math::Matrix4Ext,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    impl_component_provider,
    scene::{
        node::{Node, TypeUuidProvider},
        rigidbody::{RigidBody, RigidBodyType},
    },
    script::{ScriptContext, ScriptTrait},
};

pub mod link;

#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct Ragdoll {
    pub enabled: bool,
    #[reflect(
        description = "A handle to main actor capsule which position will be synced with root body of the ragdoll."
    )]
    #[visit(optional)]
    capsule: Handle<Node>,
    #[reflect(description = "A handle to a root body of the ragdoll.")]
    #[visit(optional)]
    root_body: Handle<Node>,
    #[visit(skip)]
    #[reflect(hidden)]
    bodies: Vec<Handle<Node>>,
    #[visit(skip)]
    #[reflect(hidden)]
    prev_enabled: bool,
}

impl_component_provider!(Ragdoll);

impl TypeUuidProvider for Ragdoll {
    fn type_uuid() -> Uuid {
        uuid!("0860b763-dfc3-46e5-8e56-32d795861d6c")
    }
}

impl ScriptTrait for Ragdoll {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        // Find all descendant rigid bodies.
        for handle in ctx.scene.graph.traverse_handle_iter(ctx.handle) {
            if ctx.scene.graph[handle].is_rigid_body() {
                self.bodies.push(handle);
            }
        }
        self.prev_enabled = self.enabled;
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        // Get linear and angular velocities of the capsule and transfer it onto rag doll bodies when it is just activated.
        let mut new_lin_vel = None;
        let mut new_ang_vel = None;
        if self.enabled && !self.prev_enabled {
            if let Some(capsule) = ctx
                .scene
                .graph
                .try_get_mut(self.capsule)
                .and_then(|n| n.cast_mut::<RigidBody>())
            {
                new_lin_vel = Some(capsule.lin_vel());
                new_ang_vel = Some(capsule.ang_vel());
            }
        }
        self.prev_enabled = self.enabled;

        for body_handle in self.bodies.iter() {
            if let Some(body) = ctx
                .scene
                .graph
                .try_get_mut(*body_handle)
                .and_then(|n| n.cast_mut::<RigidBody>())
            {
                if let Some(link) = body.script().and_then(|s| s.cast::<BoneLink>()) {
                    let bone_handle = link.bone;
                    if self.enabled {
                        // Transfer linear and angular velocities to rag doll bodies.
                        if let Some(lin_vel) = new_lin_vel {
                            body.set_lin_vel(lin_vel);
                        }
                        if let Some(ang_vel) = new_ang_vel {
                            body.set_ang_vel(ang_vel);
                        }

                        body.set_body_type(RigidBodyType::Dynamic);
                        let body_transform = body.global_transform();

                        // Sync transform of the bone with respective body.
                        let bone_parent = ctx.scene.graph[bone_handle].parent();
                        let transform: Matrix4<f32> = ctx.scene.graph[bone_parent]
                            .global_transform()
                            .try_inverse()
                            .unwrap_or_else(Matrix4::identity)
                            * body_transform;

                        ctx.scene.graph[bone_handle]
                            .local_transform_mut()
                            .set_position(Vector3::new(transform[12], transform[13], transform[14]))
                            .set_rotation(UnitQuaternion::from_matrix(&transform.basis()));
                    } else {
                        body.set_body_type(RigidBodyType::KinematicPositionBased);
                        body.set_lin_vel(Default::default());
                        body.set_ang_vel(Default::default());

                        // Sync transform of the body with respective bone.
                        if let Some(bone) = ctx.scene.graph.try_get(bone_handle) {
                            let position = bone.global_position();
                            let rotation =
                                UnitQuaternion::from_matrix(&bone.global_transform().basis());
                            ctx.scene.graph[*body_handle]
                                .local_transform_mut()
                                .set_position(position)
                                .set_rotation(rotation);
                        }
                    }
                }
            }
        }

        if self.enabled {
            if let Some(root_body) = ctx.scene.graph.try_get(self.root_body) {
                let position = root_body.global_position();
                if let Some(capsule) = ctx
                    .scene
                    .graph
                    .try_get_mut(self.capsule)
                    .and_then(|n| n.cast_mut::<RigidBody>())
                {
                    capsule.set_lin_vel(Default::default());
                    capsule.set_ang_vel(Default::default());
                    capsule.local_transform_mut().set_position(position);
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
