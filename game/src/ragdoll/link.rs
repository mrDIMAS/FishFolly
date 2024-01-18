//! A link to a bone that will be controlled by ragdoll.

use fyrox::{
    core::{
        impl_component_provider,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::node::Node,
    script::ScriptTrait,
};

#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct BoneLink {
    pub bone: Handle<Node>,
}

impl_component_provider!(BoneLink);

impl TypeUuidProvider for BoneLink {
    fn type_uuid() -> Uuid {
        uuid!("cf7729b1-6f9d-460f-a898-230a270c25be")
    }
}

impl ScriptTrait for BoneLink {}
