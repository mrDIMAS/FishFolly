//! A link to a bone that will be controlled by ragdoll.

use fyrox::{
    core::{
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    impl_component_provider,
    scene::{node::Node, node::TypeUuidProvider},
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

impl ScriptTrait for BoneLink {
    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
