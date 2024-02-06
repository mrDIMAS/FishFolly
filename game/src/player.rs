//! Main player (host) script.

use crate::{
    actor::{Actor, ActorMessage},
    net::ClientMessage,
    CameraController, Event, Game,
};
use fyrox::graph::SceneGraph;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        log::Log,
        math::SmoothAngle,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    event::{DeviceEvent, ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    scene::{camera::Camera, node::Node, rigidbody::RigidBody},
    script::{
        ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload, ScriptTrait,
    },
};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct InputController {
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub pitch: f32,
    pub yaw: f32,
}

impl InputController {
    pub fn on_os_event(&mut self, event: &Event<()>, pitch_range: &Range<f32>, dt: f32) -> bool {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            if let PhysicalKey::Code(keycode) = event.physical_key {
                let state = event.state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW => {
                        self.move_forward = state;
                        return true;
                    }
                    KeyCode::KeyS => {
                        self.move_backward = state;
                        return true;
                    }
                    KeyCode::KeyA => {
                        self.move_left = state;
                        return true;
                    }
                    KeyCode::KeyD => {
                        self.move_right = state;
                        return true;
                    }
                    KeyCode::Space => {
                        self.jump = state;
                        return true;
                    }
                    _ => (),
                }
            }
        } else if let Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } = event
        {
            self.yaw -= delta.0 as f32 * dt;
            self.pitch = (self.pitch + delta.1 as f32 * dt)
                .clamp(pitch_range.start.to_radians(), pitch_range.end.to_radians());
            return true;
        }
        false
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
    #[visit(skip)]
    #[reflect(hidden)]
    pub model_angle: SmoothAngle,
    #[reflect(description = "Pitch range for camera")]
    pitch_range: Range<f32>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            model: Default::default(),
            camera: Default::default(),
            input_controller: Default::default(),
            actor: Default::default(),
            pitch_range: -90.0f32..90.0f32,
            model_angle: SmoothAngle {
                angle: 0.0,
                target: 0.0,
                speed: 1.5 * std::f32::consts::TAU, // 540 deg/s
            },
        }
    }
}

impl ScriptTrait for Player {
    fn on_init(&mut self, ctx: &mut ScriptContext) {
        ctx.plugins
            .get_mut::<Game>()
            .level
            .actors
            .insert(ctx.handle);

        Log::info(format!(
            "Player {:?} created!",
            ctx.scene.graph[ctx.handle].instance_id()
        ));
    }

    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.message_dispatcher
            .subscribe_to::<ActorMessage>(ctx.handle);

        // Disable camera for remote players, because multiple camera will.
        if let Some(camera_controller) = ctx
            .scene
            .graph
            .try_get_script_component_of_mut::<CameraController>(self.camera)
        {
            let camera = camera_controller.camera;
            if let Some(camera) = ctx.scene.graph.try_get_mut_of_type::<Camera>(camera) {
                camera.set_enabled(!self.actor.is_remote);
            }
        }
    }

    fn on_deinit(&mut self, ctx: &mut ScriptDeinitContext) {
        ctx.plugins
            .get_mut::<Game>()
            .level
            .actors
            .remove(&ctx.node_handle);
        Log::info(format!(
            "Player {:?} destroyed!",
            ctx.scene.graph[ctx.node_handle].instance_id()
        ));
    }

    fn on_os_event(&mut self, event: &Event<()>, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get_mut::<Game>();

        if self.actor.is_remote || game.level.leaderboard.is_finished(ctx.handle) {
            return;
        }

        let this = &ctx.scene.graph[ctx.handle];
        if self
            .input_controller
            .on_os_event(event, &self.pitch_range, ctx.dt)
        {
            if let Some(client) = game.client.as_mut() {
                client.send_message_to_server(ClientMessage::Input {
                    player: this.instance_id(),
                    input_state: self.input_controller.clone(),
                })
            }
        }
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get_mut::<Game>();

        if game.is_client() || game.level.leaderboard.is_finished(ctx.handle) {
            return;
        }

        let has_ground_contact = self.actor.has_ground_contact(&ctx.scene.graph);
        let is_in_jump_state = self.actor.is_in_jump_state(&ctx.scene.graph);

        if let Some(camera_controller) = ctx
            .scene
            .graph
            .try_get_script_component_of_mut::<CameraController>(self.camera)
        {
            camera_controller.pitch = self.input_controller.pitch;
            camera_controller.yaw = self.input_controller.yaw;
        }

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

            if self.input_controller.jump
                && has_ground_contact
                && !is_in_jump_state
                && self.actor.jump_interval <= 0.0
            {
                self.actor.target_desired_velocity.y = self.actor.jump_vel;
                self.input_controller.jump = false;
                self.actor.jump();
            } else {
                self.actor.target_desired_velocity.y = 0.0;
            }

            let is_moving = rigid_body.lin_vel().xz().norm() > 0.2;

            if is_moving {
                rigid_body
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        self.input_controller.yaw,
                    ));

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

                self.model_angle.set_target(angle.to_radians());

                ctx.scene.graph[self.model]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        180.0f32.to_radians() + self.model_angle.angle(),
                    ));
            }
        }

        self.model_angle.update(ctx.dt);

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
