//! A cuboid respawn zone, any actor (player or bot) that will touch respawn zone will be spawned
//! at one of start points.

use crate::game_ref;
use fyrox::{
    core::{
        math::aabb::AxisAlignedBoundingBox,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    impl_component_provider,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct RespawnZone {}

impl_component_provider!(RespawnZone);

impl TypeUuidProvider for RespawnZone {
    fn type_uuid() -> Uuid {
        uuid!("5b39b359-0eae-4f06-958e-2facf58ce3a5")
    }
}

impl ScriptTrait for RespawnZone {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game_ref = game_ref(ctx.plugins);
        let self_bounds = AxisAlignedBoundingBox::unit()
            .transform(&ctx.scene.graph[ctx.handle].global_transform());

        let start_points = game_ref
            .start_points
            .iter()
            .map(|p| ctx.scene.graph[*p].global_position())
            .collect::<Vec<_>>();

        for actor in game_ref.actors.iter() {
            if let Some(node) = ctx.scene.graph.try_get_mut(*actor) {
                if self_bounds.is_contains_point(node.global_position()) {
                    if let Some(start_point) = start_points.first() {
                        node.local_transform_mut().set_position(*start_point);
                    }
                }
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
