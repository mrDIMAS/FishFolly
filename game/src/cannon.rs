//! Cannon shoots large balls that push players (or bots) off the platforms.

use crate::{
    net::{InstanceDescriptor, ServerMessage},
    Game,
};
use fyrox::{
    core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    resource::model::ModelResource,
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "becf5c5f-c745-40ee-85c9-491656fd222e")]
#[visit(optional)]
pub struct Cannon {
    ball_prefab: Option<ModelResource>,
    shooting_timeout: f32,
    shooting_force: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    timer: f32,
}

impl Default for Cannon {
    fn default() -> Self {
        Self {
            ball_prefab: None,
            shooting_timeout: 2.0,
            timer: 0.0,
            shooting_force: 100.0,
        }
    }
}

impl ScriptTrait for Cannon {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get_mut::<Game>();

        if let Some(server) = game.server.as_mut() {
            self.timer += ctx.dt;
            if self.timer >= self.shooting_timeout {
                self.timer = 0.0;

                let self_node = &ctx.scene.graph[ctx.handle];
                let self_position = self_node.global_position();
                let shooting_dir = self_node
                    .look_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default();
                if let Some(ball_prefab) = self.ball_prefab.as_ref() {
                    server.broadcast_message(ServerMessage::Instantiate(vec![
                        InstanceDescriptor {
                            path: ball_prefab.kind().path().unwrap().to_path_buf(),
                            position: self_position,
                            rotation: Default::default(),
                            velocity: shooting_dir.scale(self.shooting_force),
                        },
                    ]));
                }
            }
        }
    }
}
