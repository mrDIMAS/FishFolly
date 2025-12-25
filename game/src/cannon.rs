//! Cannon shoots large balls that push players (or bots) off the platforms.

use crate::{
    net::{InstanceDescriptor, ServerMessage},
    Game,
};
use fyrox::core::math::vector_to_quat;
use fyrox::plugin::error::GameResult;
use fyrox::{
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    resource::model::{ModelResource, ModelResourceExtension},
    scene::{animation::AnimationPlayer, node::Node, sound::Sound},
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "becf5c5f-c745-40ee-85c9-491656fd222e")]
#[visit(optional)]
pub struct Cannon {
    ball_prefab: InheritableVariable<Option<ModelResource>>,
    shot_effect: InheritableVariable<Option<ModelResource>>,
    shooting_force: InheritableVariable<f32>,
    shot_sound: InheritableVariable<Handle<Node>>,
    animation_player: InheritableVariable<Handle<Node>>,
}

impl Default for Cannon {
    fn default() -> Self {
        Self {
            ball_prefab: None.into(),
            shot_effect: Default::default(),
            shooting_force: 100.0.into(),
            shot_sound: Default::default(),
            animation_player: Default::default(),
        }
    }
}

impl ScriptTrait for Cannon {
    fn on_update(&mut self, ctx: &mut ScriptContext) -> GameResult {
        let game = ctx.plugins.get_mut::<Game>();
        if game.is_client() {
            return Ok(());
        }

        let Some(server) = game.server.as_mut() else {
            return Ok(());
        };

        let mbc = ctx.scene.graph.begin_multi_borrow();

        let self_node = mbc.get(ctx.handle);
        let self_position = self_node.global_position();
        let shooting_dir = self_node
            .look_vector()
            .try_normalize(f32::EPSILON)
            .unwrap_or_default();

        let mut animation_player =
            mbc.try_get_component_of_type_mut::<AnimationPlayer>(*self.animation_player)?;

        let animations = animation_player.animations_mut().get_value_mut_silent();
        if let Some(shot_animation) = animations.iter_mut().next() {
            while let Some(event) = shot_animation.pop_event() {
                if event.name == "Shoot" {
                    if let Some(ball_prefab) = self.ball_prefab.as_ref() {
                        server.broadcast_message_to_clients(ServerMessage::Instantiate(vec![
                            InstanceDescriptor {
                                path: ctx.resource_manager.resource_path(ball_prefab).unwrap(),
                                position: self_position,
                                rotation: Default::default(),
                                velocity: shooting_dir.scale(*self.shooting_force),
                                ids: ball_prefab.generate_ids(),
                            },
                        ]));
                    }

                    if let Some(shot_effect) = self.shot_effect.as_ref() {
                        server.broadcast_message_to_clients(ServerMessage::Instantiate(vec![
                            InstanceDescriptor {
                                path: ctx.resource_manager.resource_path(shot_effect).unwrap(),
                                position: self_position,
                                rotation: vector_to_quat(shooting_dir),
                                ids: shot_effect.generate_ids(),
                                ..Default::default()
                            },
                        ]));
                    }

                    let mut sound = mbc.try_get_component_of_type_mut::<Sound>(*self.shot_sound)?;
                    sound.set_playback_time(0.0);
                    sound.play();
                }
            }
        }
        Ok(())
    }
}
