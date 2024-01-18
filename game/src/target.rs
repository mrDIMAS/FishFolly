//! A target that bots will try to reach.

use crate::Game;
use fyrox::{
    core::{
        impl_component_provider, log::Log, reflect::prelude::*, type_traits::prelude::*,
        uuid::uuid, uuid::Uuid, visitor::prelude::*,
    },
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct Target {}

impl_component_provider!(Target);

impl TypeUuidProvider for Target {
    fn type_uuid() -> Uuid {
        uuid!("dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")
    }
}

impl ScriptTrait for Target {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(ctx.plugins.get_mut::<Game>().targets.insert(ctx.handle));
        Log::info(format!("Target {:?} added!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .targets
            .remove(&ctx.node_handle));
        Log::info(format!("Target {:?} destroyed!", ctx.node_handle));
    }
}
