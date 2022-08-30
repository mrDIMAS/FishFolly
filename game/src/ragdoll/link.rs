//! A link to a bone that will be controlled by ragdoll.

use crate::GameConstructor;
use fyrox::{
    core::{
        inspect::prelude::*,
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    impl_component_provider, impl_directly_inheritable_entity_trait,
    scene::{graph::map::NodeHandleMap, node::Node, node::TypeUuidProvider},
    script::ScriptTrait,
};

#[derive(Clone, Default, Debug, Visit, Inspect, Reflect)]
pub struct BoneLink {
    pub bone: Handle<Node>,
}

impl_component_provider!(BoneLink);
impl_directly_inheritable_entity_trait!(BoneLink;);

impl TypeUuidProvider for BoneLink {
    fn type_uuid() -> Uuid {
        uuid!("cf7729b1-6f9d-460f-a898-230a270c25be")
    }
}

impl ScriptTrait for BoneLink {
    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping.map(&mut self.bone);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        GameConstructor::type_uuid()
    }
}
