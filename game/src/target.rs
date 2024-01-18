//! A target that bots will try to reach.

use crate::Game;
use fyrox::{
    core::{log::Log, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")]
#[visit(optional)]
pub struct Target {}

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
