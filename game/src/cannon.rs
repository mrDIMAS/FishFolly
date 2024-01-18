//! Cannon shoots large balls that push players (or bots) off the platforms.

use fyrox::{
    core::{
        impl_component_provider,
        log::Log,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    resource::model::{ModelResource, ModelResourceExtension},
    scene::rigidbody::RigidBody,
    script::{ScriptContext, ScriptTrait},
};

#[derive(Clone, Debug, Visit, Reflect)]
pub struct Cannon {
    ball_prefab: Option<ModelResource>,
    shooting_timeout: f32,
    #[visit(optional)]
    shooting_force: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    timer: f32,
}

impl_component_provider!(Cannon);

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

impl TypeUuidProvider for Cannon {
    fn type_uuid() -> Uuid {
        uuid!("becf5c5f-c745-40ee-85c9-491656fd222e")
    }
}

impl ScriptTrait for Cannon {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
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
                let ball_instance = ball_prefab.instantiate(ctx.scene);
                ctx.scene.graph[ball_instance].set_lifetime(Some(5.0));

                let body = ctx
                    .scene
                    .graph
                    .find(ball_instance, &mut |node| node.tag() == "Body")
                    .map(|(h, _)| h)
                    .unwrap_or_default();
                if let Some(body) = ctx.scene.graph.try_get_mut(body) {
                    body.local_transform_mut().set_position(self_position);

                    if let Some(rigid_body) = body.cast_mut::<RigidBody>() {
                        rigid_body.set_lin_vel(shooting_dir.scale(self.shooting_force));
                    }
                } else {
                    Log::warn("Cannot find Body of ball!");
                }
            }
        }
    }
}
