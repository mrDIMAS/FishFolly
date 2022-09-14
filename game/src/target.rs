//! A target that bots will try to reach.

use crate::game_mut;
use fyrox::{
    core::{inspect::prelude::*, reflect::Reflect, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    impl_component_provider,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Default, Debug, Visit, Inspect, Reflect)]
pub struct Target {}

impl_component_provider!(Target);

impl TypeUuidProvider for Target {
    fn type_uuid() -> Uuid {
        uuid!("dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")
    }
}

impl ScriptTrait for Target {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(game_mut(ctx.plugins).targets.insert(ctx.handle));
        Log::info(format!("Target {:?} added!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(game_mut(ctx.plugins).targets.remove(&ctx.node_handle));
        Log::info(format!("Target {:?} destroyed!", ctx.node_handle));
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
