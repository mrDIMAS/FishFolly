//! A spawn point for players (bots).

use crate::Game;
use fyrox::{
    core::{log::Log, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    resource::model::{ModelResource, ModelResourceExtension},
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "103ac5c1-f4e4-45d2-a9f1-0da98d74d64c")]
#[visit(optional)]
pub struct StartPoint {
    #[reflect(
        description = "A handle of a player resource. The resource will be instantiated to the scene."
    )]
    model: Option<ModelResource>,
}

impl ScriptTrait for StartPoint {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .start_points
            .insert(ctx.handle));

        if let Some(resource) = self.model.as_ref() {
            let position = ctx.scene.graph[ctx.handle].global_position();
            // Spawn specified actor.
            resource.instantiate_at(ctx.scene, position, Default::default());
        }

        Log::info(format!("Start point {:?} created!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .start_points
            .remove(&ctx.node_handle));
        Log::info(format!("Start point {:?} destroyed!", ctx.node_handle));
    }
}
