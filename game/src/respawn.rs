//! A cuboid respawn zone, any actor (player or bot) that will touch respawn zone will be spawned
//! at one of start points.

use crate::{game_ref, Game, Uuid};
use fyrox::{
    core::{inspect::prelude::*, uuid::uuid, visitor::prelude::*},
    impl_component_provider,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct RespawnZone {}

impl_component_provider!(RespawnZone);

impl TypeUuidProvider for RespawnZone {
    fn type_uuid() -> Uuid {
        uuid!("5b39b359-0eae-4f06-958e-2facf58ce3a5")
    }
}

impl ScriptTrait for RespawnZone {
    fn on_update(&mut self, context: ScriptContext) {
        let self_bounds = context.scene.graph[context.handle].world_bounding_box();

        for actor in game_ref(context.plugin).actors.iter() {
            if let Some(node) = context.scene.graph.try_get(*actor) {
                if self_bounds.is_contains_point(node.global_position()) {
                    // TODO: Respawn.
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
