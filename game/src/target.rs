//! A target that bots will try to reach.

use crate::{Game, Uuid};
use fyrox::{
    core::{inspect::prelude::*, uuid::uuid, visitor::prelude::*},
    scene::node::TypeUuidProvider,
    script::ScriptTrait,
};

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct Target {}

impl TypeUuidProvider for Target {
    fn type_uuid() -> Uuid {
        uuid!("dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")
    }
}

impl ScriptTrait for Target {
    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
