//! Jumper is platform that pushes actors (players or bots) up.

use crate::{Bot, Game, Player};
use fyrox::graph::SceneGraph;
use fyrox::plugin::error::GameResult;
use fyrox::{
    core::{
        algebra::Vector3, reflect::prelude::*, type_traits::prelude::*,
        variable::InheritableVariable, visitor::prelude::*,
    },
    scene::{collider::Collider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
};
use std::collections::HashSet;

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "be8a29af-c10a-4518-a78b-955c8f48a8cd")]
#[visit(optional)]
pub struct Jumper {
    push_force: InheritableVariable<f32>,
}

impl ScriptTrait for Jumper {
    fn on_update(&mut self, ctx: &mut ScriptContext) -> GameResult {
        let game = ctx.plugins.get::<Game>();
        if game.is_client() {
            return Ok(());
        }

        let collider = ctx.scene.graph.try_get_of_type::<Collider>(ctx.handle)?;

        let mut contacted_colliders = HashSet::new();

        for contact in collider.contacts(&ctx.scene.graph.physics) {
            for actor in game.level.actors.iter() {
                let actor_script = ctx.scene.graph[*actor].script(0);

                if let Some(actor_collider) = actor_script
                    .and_then(|s| s.query_component_ref::<Player>().map(|p| p.actor.collider))
                    .or_else(|| {
                        actor_script
                            .and_then(|s| s.query_component_ref::<Bot>().map(|b| b.actor.collider))
                    })
                {
                    if contact.collider1 == actor_collider || contact.collider2 == actor_collider {
                        contacted_colliders.insert(actor_collider);
                    }
                }
            }
        }

        for collider in contacted_colliders {
            let parent = ctx.scene.graph.try_get(collider)?.parent();
            let rigid_body = ctx.scene.graph.try_get_mut_of_type::<RigidBody>(parent)?;
            let lin_vel = rigid_body.lin_vel();
            rigid_body.set_lin_vel(Vector3::new(lin_vel.x, *self.push_force, lin_vel.z));
        }

        Ok(())
    }
}
