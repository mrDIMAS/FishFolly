use crate::{actor::Actor, Game};
use fyrox::graph::BaseSceneGraph;
use fyrox::{
    core::{
        math::aabb::AxisAlignedBoundingBox, reflect::prelude::*, type_traits::prelude::*,
        variable::InheritableVariable, visitor::prelude::*,
    },
    script::{ScriptContext, ScriptTrait},
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
#[type_uuid(id = "5b39b359-0eae-4f06-955e-2facf58ce3a2")]
pub enum Action {
    #[default]
    Finish,
}

#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "a87db06a-0e59-4af3-a6be-fb1432a147cc")]
#[visit(optional)]
pub struct Trigger {
    action: InheritableVariable<Action>,
    lulw: i32,
    azaza: i32,
}

impl ScriptTrait for Trigger {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get_mut::<Game>();
        if game.is_client() {
            return;
        }

        let this = &ctx.scene.graph[ctx.handle];
        let self_bounds = AxisAlignedBoundingBox::unit().transform(&this.global_transform());

        for actor_handle in game.level.actors.iter() {
            if let Some(actor_script) = ctx
                .scene
                .graph
                .try_get_script_component_of::<Actor>(*actor_handle)
            {
                let rigid_body = actor_script.rigid_body;
                if let Some(rigid_body) = ctx.scene.graph.try_get(rigid_body) {
                    if self_bounds.is_contains_point(rigid_body.global_position()) {
                        match *self.action {
                            Action::Finish => game.level.leaderboard.finish(*actor_handle),
                        }
                    }
                }
            }
        }
    }
}
