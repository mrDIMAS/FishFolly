//! Main player (host) script.

use crate::{game_mut, marker::Actor, utils, CameraController, Event};
use fyrox::{
    animation::machine::Parameter,
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    event::{ElementState, VirtualKeyCode, WindowEvent},
    impl_component_provider,
    scene::{
        animation::absm::AnimationBlendingStateMachine,
        graph::Graph,
        node::{Node, TypeUuidProvider},
        rigidbody::RigidBody,
    },
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    utils::log::Log,
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
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } = event
        {
            if let Some(keycode) = input.virtual_keycode {
                let state = input.state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W => self.move_forward = state,
                    VirtualKeyCode::S => self.move_backward = state,
                    VirtualKeyCode::A => self.move_left = state,
                    VirtualKeyCode::D => self.move_right = state,
                    VirtualKeyCode::Space => self.jump = state,
                    _ => (),
                }
            }
        }
    }
}

#[derive(Clone, Visit, Debug, Reflect)]
pub struct Player {
    #[reflect(description = "Speed of the player.")]
    speed: f32,
    #[reflect(description = "Handle to player's collider.")]
    pub collider: Handle<Node>,
    #[reflect(description = "Handle to player's model.")]
    model: Handle<Node>,
    #[reflect(description = "Handle to player's animation state machine.")]
    absm: Handle<Node>,
    #[reflect(description = "Handle to a node with camera controller.")]
    #[visit(optional)]
    camera: Handle<Node>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub input_controller: InputController,
    #[visit(skip)]
    #[reflect(hidden)]
    pub actor: Actor,
}

impl_component_provider!(Player, actor: Actor);

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 1.0,
            collider: Default::default(),
            model: Default::default(),
            absm: Default::default(),
            camera: Default::default(),
            input_controller: Default::default(),
            actor: Default::default(),
        }
    }
}

impl TypeUuidProvider for Player {
    fn type_uuid() -> Uuid {
        uuid!("deb77c1d-668d-4716-a8f7-04ed09b0b9f6")
    }
}

impl Player {
    pub fn has_ground_contact(&self, graph: &Graph) -> bool {
        utils::has_ground_contact(self.collider, graph)
    }
}

impl ScriptTrait for Player {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        assert!(game_mut(ctx.plugins).actors.insert(ctx.handle));

        Log::info(format!("Player {:?} created!", ctx.handle));
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        assert!(game_mut(ctx.plugins).actors.remove(&ctx.node_handle));
        Log::info(format!("Player {:?} destroyed!", ctx.node_handle));
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        self.input_controller.on_os_event(event);
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let has_ground_contact = self.has_ground_contact(&ctx.scene.graph);

        let yaw = ctx
            .scene
            .graph
            .try_get(self.camera)
            .and_then(|c| c.script())
            .and_then(|s| s.cast::<CameraController>())
            .map(|c| c.yaw)
            .unwrap_or_default();

        if let Some(rigid_body) = ctx.scene.graph[ctx.handle].cast_mut::<RigidBody>() {
            let forward_vec = rigid_body.look_vector();
            let side_vec = rigid_body.side_vector();

            let mut velocity = Vector3::default();

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

            let mut jump = false;
            if self.input_controller.jump && has_ground_contact {
                velocity.y += 5.5;
                self.input_controller.jump = false;
                jump = true;
            }

            rigid_body.set_lin_vel(velocity);

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
                        (angle + 180.0).to_radians(),
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
                    .set_parameter("Jump", Parameter::Rule(jump));
            }
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
