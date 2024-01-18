//! A spawn point for players (bots).

use crate::Game;
use fyrox::{
    core::{
        impl_component_provider, log::Log, reflect::prelude::*, uuid::uuid, uuid::Uuid,
        visitor::prelude::*, TypeUuidProvider,
    },
    resource::model::{ModelResource, ModelResourceExtension},
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};

#[derive(Clone, Default, Debug, Visit, Reflect)]
pub struct StartPoint {
    #[reflect(
        description = "A handle of a player resource. The resource will be instantiated to the scene."
    )]
    model: Option<ModelResource>,
}

impl_component_provider!(StartPoint);

impl TypeUuidProvider for StartPoint {
    fn type_uuid() -> Uuid {
        uuid!("103ac5c1-f4e4-45d2-a9f1-0da98d74d64c")
    }
}

impl ScriptTrait for StartPoint {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .start_points
            .insert(ctx.handle));

        if let Some(resource) = self.model.as_ref() {
            // Spawn specified actor.
            let instance = resource.instantiate(ctx.scene);
            // Sync its position with the start point position.
            let position = ctx.scene.graph[ctx.handle].global_position();
            let body = ctx
                .scene
                .graph
                .find(instance, &mut |node| node.tag() == "Body")
                .map(|(h, _)| h)
                .unwrap_or_default();
            if let Some(body) = ctx.scene.graph.try_get_mut(body) {
                body.local_transform_mut().set_position(position);
            } else {
                Log::warn("Cannot find Body of actor!");
            }
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
