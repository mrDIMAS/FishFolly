//! A spawn point for players (bots).

use crate::{game_mut, Game, Message, Uuid};
use fyrox::{
    core::{inspect::prelude::*, pool::Handle, uuid::uuid, visitor::prelude::*},
    engine::resource_manager::ResourceManager,
    gui::inspector::PropertyChanged,
    handle_object_property_changed, impl_component_provider,
    resource::model::Model,
    scene::node::{Node, TypeUuidProvider},
    script::{ScriptContext, ScriptTrait},
    utils::log::Log,
};
use std::sync::mpsc::Sender;

#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct StartPoint {
    #[inspect(
        description = "A handle of a player resource. The resource will be instantiated to the scene."
    )]
    model: Option<Model>,

    #[visit(skip)]
    #[inspect(skip)]
    self_handle: Handle<Node>,

    #[visit(skip)]
    #[inspect(skip)]
    sender: Option<Sender<Message>>,
}

impl Drop for StartPoint {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.as_ref() {
            sender
                .send(Message::UnregisterStartPoint(self.self_handle))
                .unwrap();
        }
    }
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
        let game = game_mut(context.plugin);
        self.self_handle = context.handle;
        self.sender = Some(game.message_sender.clone());
        game.start_points.insert(context.handle);

        if let Some(resource) = self.model.as_ref() {
            let instance = resource.instantiate_geometry(context.scene);

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
