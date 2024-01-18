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
    scene::{animation::absm::prelude::*, node::Node, rigidbody::RigidBody},
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
    #[reflect(description = "Speed of the player.")]
    speed: f32,
    #[reflect(description = "Handle to player's model.")]
    model: Handle<Node>,
    #[reflect(description = "Handle to player's animation state machine.")]
    absm: Handle<Node>,
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
            speed: 1.0,
            model: Default::default(),
            absm: Default::default(),
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
        self.actor.on_update(ctx);

        let has_ground_contact = self.actor.has_ground_contact(&ctx.scene.graph);

        let yaw = ctx
            .scene
            .graph
            .try_get(self.camera)
            .and_then(|c| c.script())
            .and_then(|s| s.cast::<CameraController>())
            .map(|c| c.yaw)
            .unwrap_or_default();

        let mut velocity = Vector3::default();

        if let Some(rigid_body) = ctx.scene.graph[self.actor.rigid_body].cast_mut::<RigidBody>() {
            let forward_vec = rigid_body.look_vector();
            let side_vec = rigid_body.side_vector();

            if self.input_controller.move_forward {
                velocity += forward_vec;
            }
            if self.input_controller.move_backward {
                velocity -= forward_vec;
            }
            if self.input_controller.move_left {
                velocity += side_vec;
            }
            if self.input_controller.move_right {
                velocity -= side_vec;
            }

            velocity = velocity
                .try_normalize(f32::EPSILON)
                .map(|v| v.scale(self.speed))
                .unwrap_or_default();

            velocity.y = rigid_body.lin_vel().y;

            if self.input_controller.jump && has_ground_contact {
                velocity.y += 5.5;
                self.input_controller.jump = false;
                self.actor.jump = true;
            }

            let is_moving = velocity.x != 0.0 || velocity.z != 0.0;

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

            if let Some(absm) = ctx
                .scene
                .graph
                .try_get_mut(self.absm)
                .and_then(|n| n.query_component_mut::<AnimationBlendingStateMachine>())
            {
                absm.machine_mut()
                    .get_value_mut_silent()
                    .set_parameter("Run", Parameter::Rule(is_moving))
                    .set_parameter("Jump", Parameter::Rule(self.actor.jump));
            }
        }

        self.actor
            .do_move(velocity, &mut ctx.scene.graph, has_ground_contact);
    }

    fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) {
        self.actor.on_message(message, ctx);
    }
}
