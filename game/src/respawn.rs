//! A cuboid respawn zone, any actor (player or bot) that will touch respawn zone will be spawned
//! at one of start points.

use crate::{
    actor::{Actor, ActorMessage},
    Game,
};
use fyrox::{
    core::{
        math::aabb::AxisAlignedBoundingBox, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    rand::{seq::SliceRandom, thread_rng},
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5b39b359-0eae-4f06-958e-2facf58ce3a5")]
#[visit(optional)]
pub struct RespawnZone {}

impl ScriptTrait for RespawnZone {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        if game.is_client() {
            return;
        }

        let self_bounds = AxisAlignedBoundingBox::unit()
            .transform(&ctx.scene.graph[ctx.handle].global_transform());

        let start_points = game
            .start_points
            .iter()
            .map(|p| ctx.scene.graph[*p].global_position())
            .collect::<Vec<_>>();

        for actor_handle in game.actors.iter() {
            if let Some(actor_script) = ctx
                .scene
                .graph
                .try_get_script_component_of::<Actor>(*actor_handle)
            {
                let rigid_body = actor_script.rigid_body;
                if let Some(rigid_body) = ctx.scene.graph.try_get(rigid_body) {
                    if self_bounds.is_contains_point(rigid_body.global_position()) {
                        if let Some(start_point) = start_points.choose(&mut thread_rng()) {
                            ctx.message_sender.send_to_target(
                                *actor_handle,
                                ActorMessage::RespawnAt(*start_point),
                            );
                        }
                    }
                }
            }
        }
    }
}
