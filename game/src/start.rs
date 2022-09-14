//! A spawn point for players (bots).

use crate::game_mut;
use fyrox::{
    core::{inspect::prelude::*, reflect::Reflect, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    engine::resource_manager::ResourceManager,
    impl_component_provider,
    resource::model::Model,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Default, Debug, Visit, Inspect, Reflect)]
pub struct StartPoint {
    #[inspect(
        description = "A handle of a player resource. The resource will be instantiated to the scene."
    )]
    model: Option<Model>,
}

impl_component_provider!(StartPoint);

impl TypeUuidProvider for StartPoint {
    fn type_uuid() -> Uuid {
        uuid!("103ac5c1-f4e4-45d2-a9f1-0da98d74d64c")
    }
}

impl ScriptTrait for StartPoint {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(game_mut(ctx.plugins).start_points.insert(ctx.handle));

        if let Some(resource) = self.model.as_ref() {
            // Spawn specified actor.
            let instance = resource.instantiate_geometry(ctx.scene);
            // Sync its position with the start point position.
            let position = ctx.scene.graph[ctx.handle].global_position();
            let body = ctx
                .scene
                .graph
                .find(instance, &mut |node| node.tag() == "Body");
            if let Some(body) = ctx.scene.graph.try_get_mut(body) {
                body.local_transform_mut().set_position(position);
            } else {
                Log::warn("Cannot find Body of actor!".to_owned());
            }
        }

        Log::info(format!("Start point {:?} created!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(game_mut(ctx.plugins).start_points.remove(&ctx.node_handle));
        Log::info(format!("Start point {:?} destroyed!", ctx.node_handle));
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        resource_manager
            .state()
            .containers_mut()
            .models
            .try_restore_optional_resource(&mut self.model);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
