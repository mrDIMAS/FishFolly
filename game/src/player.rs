//! Main player (host) script.

use crate::{actor::Actor, actor::ActorMessage, CameraController, Event, Game};
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        log::Log,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    scene::{node::Node, rigidbody::RigidBody},
    script::{
        ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload, ScriptTrait,
    },
};

#[derive(Clone, Default, Debug)]
pub struct InputController {
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
}

impl InputController {
    pub fn on_os_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            if let PhysicalKey::Code(keycode) = event.physical_key {
                let state = event.state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW => self.move_forward = state,
                    KeyCode::KeyS => self.move_backward = state,
                    KeyCode::KeyA => self.move_left = state,
                    KeyCode::KeyD => self.move_right = state,
                    KeyCode::Space => self.jump = state,
                    _ => (),
                }
            }
        }
    }
}

#[derive(Clone, Visit, Debug, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "deb77c1d-668d-4716-a8f7-04ed09b0b9f6")]
#[visit(optional)]
pub struct Player {
    #[reflect(description = "Handle to player's model.")]
    model: Handle<Node>,
    #[reflect(description = "Handle to a node with camera controller.")]
    camera: Handle<Node>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub input_controller: InputController,
    #[component(include)]
    pub actor: Actor,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            model: Default::default(),
            camera: Default::default(),
            input_controller: Default::default(),
            actor: Default::default(),
        }
    }
}

impl ScriptTrait for Player {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(ctx.plugins.get_mut::<Game>().actors.insert(ctx.handle));

        Log::info(format!("Player {:?} created!", ctx.handle));
    }

    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.message_dispatcher
            .subscribe_to::<ActorMessage>(ctx.handle);
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(ctx
            .plugins
            .get_mut::<Game>()
            .actors
            .remove(&ctx.node_handle));
        Log::info(format!("Player {:?} destroyed!", ctx.node_handle));
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        self.input_controller.on_os_event(event);
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let has_ground_contact = self.actor.has_ground_contact(&ctx.scene.graph);

        let yaw = ctx
            .scene
            .graph
            .try_get(self.camera)
            .and_then(|c| c.script())
            .and_then(|s| s.cast::<CameraController>())
            .map(|c| c.yaw)
            .unwrap_or_default();

        self.actor.target_desired_velocity = Vector3::default();

        if let Some(rigid_body) = ctx.scene.graph[self.actor.rigid_body].cast_mut::<RigidBody>() {
            let forward_vec = rigid_body.look_vector();
            let side_vec = rigid_body.side_vector();

            if self.input_controller.move_forward {
                self.actor.target_desired_velocity += forward_vec;
            }
            if self.input_controller.move_backward {
                self.actor.target_desired_velocity -= forward_vec;
            }
            if self.input_controller.move_left {
                self.actor.target_desired_velocity += side_vec;
            }
            if self.input_controller.move_right {
                self.actor.target_desired_velocity -= side_vec;
            }

            self.actor.target_desired_velocity = self
                .actor
                .target_desired_velocity
                .try_normalize(f32::EPSILON)
                .map(|v| v.scale(self.actor.speed))
                .unwrap_or_default();

            if self.input_controller.jump && has_ground_contact {
                self.actor.target_desired_velocity.y = self.actor.jump_vel;
                self.input_controller.jump = false;
                self.actor.jump = true;
            } else {
                self.actor.target_desired_velocity.y = 0.0;
            }

            let is_moving = rigid_body.lin_vel().xz().norm() > 0.2;

            if is_moving {
                rigid_body
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw));

                // Apply additional rotation to model - it will turn in front of walking direction.
                let angle: f32 = if self.input_controller.move_left {
                    if self.input_controller.move_forward {
                        45.0
                    } else if self.input_controller.move_backward {
                        135.0
                    } else {
                        90.0
                    }
                } else if self.input_controller.move_right {
                    if self.input_controller.move_forward {
                        -45.0
                    } else if self.input_controller.move_backward {
                        -135.0
                    } else {
                        -90.0
                    }
                } else if self.input_controller.move_backward {
                    180.0
                } else {
                    0.0
                };

                ctx.scene.graph[self.model]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        (180.0 + angle).to_radians(),
                    ));
            }
        }

        self.actor.on_update(ctx);
    }

    fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) {
        self.actor.on_message(message, ctx);
    }
}
