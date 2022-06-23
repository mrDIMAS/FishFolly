//! A target that bots will try to reach.

use crate::{game_mut, Game};
use fyrox::{
    core::{inspect::prelude::*, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    impl_component_provider,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct Target {}

impl_component_provider!(Target);

impl TypeUuidProvider for Target {
    fn type_uuid() -> Uuid {
        uuid!("dcf159d1-6bd9-4e19-8a2a-c838a1ab8f0d")
    }
}

impl ScriptTrait for Target {
    fn on_init(&mut self, context: ScriptContext) {
        let game = game_mut(context.plugin);
        game.targets.insert(context.handle);
    }

    fn on_deinit(&mut self, context: ScriptDeinitContext) {
        assert!(game_mut(context.plugin)
            .targets
            .remove(&context.node_handle));
        Log::info(format!("Target {:?} destroyed!", context.node_handle));
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
