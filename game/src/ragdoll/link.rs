//! A link to a bone that will be controlled by ragdoll.

use crate::GameConstructor;
use fyrox::{
    core::{
        inspect::prelude::*,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    scene::{graph::map::NodeHandleMap, node::Node, node::TypeUuidProvider},
    script::ScriptTrait,
};

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct BoneLink {
    pub bone: Handle<Node>,
}

impl_component_provider!(BoneLink);

impl TypeUuidProvider for BoneLink {
    fn type_uuid() -> Uuid {
        uuid!("cf7729b1-6f9d-460f-a898-230a270c25be")
    }
}

impl ScriptTrait for BoneLink {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args, Self::BONE=> bone)
    }

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
