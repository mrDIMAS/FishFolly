//! A cuboid respawn zone, any actor (player or bot) that will touch respawn zone will be spawned
//! at one of start points.

use crate::{
    actor::{Actor, ActorMessage},
    Game,
};
use fyrox::plugin::error::GameResult;
use fyrox::{
    core::{
        math::aabb::AxisAlignedBoundingBox, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, variable::InheritableVariable, visitor::prelude::*,
    },
    graph::SceneGraph,
    rand::{seq::SliceRandom, thread_rng},
    scene::{collider::Collider, node::Node},
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[derive(
    Default,
    Clone,
    Debug,
    PartialEq,
    Visit,
    Reflect,
    TypeUuidProvider,
    AsRefStr,
    EnumString,
    VariantNames,
)]
#[type_uuid(id = "5b39b359-0eae-4f06-958e-2facf58ce3a2")]
pub enum RespawnMode {
    #[default]
    OnEnterBoundingBox,
    OnContact,
    Disabled,
}

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5b39b359-0eae-4f06-958e-2facf58ce3a5")]
#[visit(optional)]
pub struct Respawner {
    mode: InheritableVariable<RespawnMode>,
    pub collider: InheritableVariable<Handle<Node>>,
}

impl ScriptTrait for Respawner {
    fn on_start(&mut self, ctx: &mut ScriptContext) -> GameResult {
        ctx.plugins
            .get_mut::<Game>()
            .level
            .respawners
            .insert(ctx.handle);
        Ok(())
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) -> GameResult {
        ctx.plugins
            .get_mut::<Game>()
            .level
            .respawners
            .remove(&ctx.node_handle);
        Ok(())
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) -> GameResult {
        let game = ctx.plugins.get::<Game>();
        if game.is_client() {
            return Ok(());
        }

        let self_bounds = AxisAlignedBoundingBox::unit()
            .transform(&ctx.scene.graph[ctx.handle].global_transform());

        let start_points = game
            .level
            .start_points
            .iter()
            .map(|p| ctx.scene.graph[*p].global_position())
            .collect::<Vec<_>>();

        for actor_handle in game.level.actors.iter() {
            let actor_script = ctx
                .scene
                .graph
                .try_get_script_component_of::<Actor>(*actor_handle)?;

            match *self.mode {
                RespawnMode::OnEnterBoundingBox => {
                    let rigid_body = actor_script.rigid_body;
                    let rigid_body = ctx.scene.graph.try_get(rigid_body)?;
                    if self_bounds.is_contains_point(rigid_body.global_position()) {
                        if let Some(start_point) = start_points.choose(&mut thread_rng()) {
                            ctx.message_sender.send_to_target(
                                *actor_handle,
                                ActorMessage::RespawnAt(*start_point),
                            );
                        }
                    }
                }
                RespawnMode::OnContact => {
                    let collider = ctx
                        .scene
                        .graph
                        .try_get_of_type::<Collider>(*self.collider)?;
                    for contact in collider.contacts(&ctx.scene.graph.physics) {
                        if contact.has_any_active_contact
                            && (contact.collider1 == actor_script.collider
                                || contact.collider2 == actor_script.collider)
                        {
                            if let Some(start_point) = start_points.choose(&mut thread_rng()) {
                                ctx.message_sender.send_to_target(
                                    *actor_handle,
                                    ActorMessage::RespawnAt(*start_point),
                                );
                            }
                        }
                    }
                }
                RespawnMode::Disabled => {}
            }
        }

        Ok(())
    }
}
