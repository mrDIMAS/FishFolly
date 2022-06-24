//! A spawn point for players (bots).

use crate::{game_mut, Game};
use fyrox::{
    core::{inspect::prelude::*, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    engine::resource_manager::ResourceManager,
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    resource::model::Model,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Default, Debug, Visit, Inspect)]
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
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args, Self::MODEL => model)
    }

    fn on_init(&mut self, context: ScriptContext) {
        assert!(game_mut(context.plugin).start_points.insert(context.handle));

        if let Some(resource) = self.model.as_ref() {
            // Spawn specified actor.
            let instance = resource.instantiate_geometry(context.scene);
            // Sync its position with the start point position.
            let position = context.scene.graph[context.handle].global_position();
            let body = context
                .scene
                .graph
                .find(instance, &mut |node| node.tag() == "Body");
            if let Some(body) = context.scene.graph.try_get_mut(body) {
                body.local_transform_mut().set_position(position);
            } else {
                Log::warn("Cannot find Body of actor!".to_owned());
            }
        }

        Log::info(format!("Start point {:?} created!", context.handle));
    }

    fn on_deinit(&mut self, context: ScriptDeinitContext) {
        assert!(game_mut(context.plugin)
            .start_points
            .remove(&context.node_handle));
        Log::info(format!("Start point {:?} destroyed!", context.node_handle));
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

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
