//! A ragdoll that is used for characters that fall.
use crate::{ragdoll::link::BoneLink, GameConstructor};
use fyrox::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        inspect::prelude::*,
        math::Matrix4Ext,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    scene::{
        graph::map::NodeHandleMap,
        node::{Node, TypeUuidProvider},
        rigidbody::{RigidBody, RigidBodyType},
    },
    script::{ScriptContext, ScriptTrait},
};

pub mod link;

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct Ragdoll {
    pub enabled: bool,
    #[inspect(
        description = "A handle to main actor capsule which position will be synced with root body of the ragdoll."
    )]
    #[visit(optional)]
    capsule: Handle<Node>,
    #[inspect(description = "A handle to a root body of the ragdoll.")]
    #[visit(optional)]
    root_body: Handle<Node>,
    #[visit(skip)]
    #[inspect(skip)]
    bodies: Vec<Handle<Node>>,
    #[visit(skip)]
    #[inspect(skip)]
    prev_enabled: bool,
}

impl_component_provider!(Ragdoll);

impl TypeUuidProvider for Ragdoll {
    fn type_uuid() -> Uuid {
        uuid!("0860b763-dfc3-46e5-8e56-32d795861d6c")
    }
}

impl ScriptTrait for Ragdoll {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args,
            Self::ENABLED => enabled,
            Self::CAPSULE => capsule,
            Self::ROOT_BODY => root_body
        )
    }

    fn on_init(&mut self, context: ScriptContext) {
        // Find all descendant rigid bodies.
        for handle in context.scene.graph.traverse_handle_iter(context.handle) {
            if context.scene.graph[handle].is_rigid_body() {
                self.bodies.push(handle);
            }
        }
        self.prev_enabled = self.enabled;
    }

    fn on_update(&mut self, context: ScriptContext) {
        // Get linear and angular velocities of the capsule and transfer it onto rag doll bodies when it is just activated.
        let mut new_lin_vel = None;
        let mut new_ang_vel = None;
        if self.enabled && !self.prev_enabled {
            if let Some(capsule) = context
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
            if let Some(body) = context
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
                        let bone_parent = context.scene.graph[bone_handle].parent();
                        let transform: Matrix4<f32> = context.scene.graph[bone_parent]
                            .global_transform()
                            .try_inverse()
                            .unwrap_or_else(Matrix4::identity)
                            * body_transform;

                        context.scene.graph[bone_handle]
                            .local_transform_mut()
                            .set_position(Vector3::new(transform[12], transform[13], transform[14]))
                            .set_rotation(UnitQuaternion::from_matrix(&transform.basis()));
                    } else {
                        body.set_body_type(RigidBodyType::KinematicPositionBased);
                        body.set_lin_vel(Default::default());
                        body.set_ang_vel(Default::default());

                        // Sync transform of the body with respective bone.
                        if let Some(bone) = context.scene.graph.try_get(bone_handle) {
                            let position = bone.global_position();
                            let rotation =
                                UnitQuaternion::from_matrix(&bone.global_transform().basis());
                            context.scene.graph[*body_handle]
                                .local_transform_mut()
                                .set_position(position)
                                .set_rotation(rotation);
                        }
                    }
                }
            }
        }

        if self.enabled {
            if let Some(root_body) = context.scene.graph.try_get(self.root_body) {
                let position = root_body.global_position();
                if let Some(capsule) = context
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

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping
            .map(&mut self.capsule)
            .map(&mut self.root_body);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}
