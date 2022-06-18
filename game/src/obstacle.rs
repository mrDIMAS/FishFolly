use crate::Game;
use fyrox::{
    core::{inspect::prelude::*, uuid::uuid, uuid::Uuid, visitor::prelude::*},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    scene::node::TypeUuidProvider,
    script::{ScriptContext, ScriptTrait},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[derive(Clone, Debug, Visit, Inspect, AsRefStr, EnumString, EnumVariantNames)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Default for Axis {
    fn default() -> Self {
        Self::X
    }
}

#[derive(Clone, Debug, Visit, Inspect, AsRefStr, EnumString, EnumVariantNames)]
pub enum ObstacleKind {
    Rotator { angle: f32, axis: Axis },
}

impl Default for ObstacleKind {
    fn default() -> Self {
        Self::Rotator {
            angle: 0.0,
            axis: Default::default(),
        }
    }
}

/// TODO: Ideally any animation for obstacles should be done in the editor, but there is no
/// animation editor yet.
#[derive(Clone, Default, Debug, Visit, Inspect)]
pub struct Obstacle {
    kind: ObstacleKind,
}

impl TypeUuidProvider for Obstacle {
    fn type_uuid() -> Uuid {
        uuid!("54ce703d-a56c-4534-a8a8-33ee1c6dd0a2")
    }
}

impl ScriptTrait for Obstacle {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args, Self::KIND => kind)
    }

    fn on_update(&mut self, _context: ScriptContext) {}

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
