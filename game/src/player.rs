//! Main player (host) script.

use crate::actor::ActorKind;
use crate::net::{InstanceDescriptor, ServerMessage};
use crate::{
    actor::{Actor, ActorMessage},
    net::ClientMessage,
    CameraController, Event, Game,
};
use fyrox::core::futures::executor::block_on;
use fyrox::resource::model::{Model, ModelResourceExtension};
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
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    graph::{BaseSceneGraph, SceneGraph},
    keyboard::{KeyCode, PhysicalKey},
    scene::{camera::Camera, node::Node, rigidbody::RigidBody},
    script::{
        ScriptContext, ScriptDeinitContext, ScriptMessageContext, ScriptMessagePayload, ScriptTrait,
    },
};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Default, Debug, Visit, Serialize, Deserialize)]
pub struct InputController {
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub target_pitch: f32,
    pub target_yaw: f32,
}

impl InputController {
    pub fn on_os_event(
        &mut self,
        event: &Event<()>,
        pitch_range: &Range<f32>,
        dt: f32,
        mouse_sens: f32,
        game: &Game,
        spectator_target: &mut Handle<Node>,
    ) -> bool {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
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
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left && *state == ElementState::Pressed {
                        for actor in &game.level.actors {
                            if *actor != *spectator_target {
                                *spectator_target = *actor;
                                break;
                            }
                        }
                    }
                }
                _ => (),
            }
        } else if let Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } = event
        {
            self.target_yaw -= delta.0 as f32 * mouse_sens * dt;
            self.target_pitch = (self.target_pitch + delta.1 as f32 * mouse_sens * dt)
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
    #[reflect(hidden)]
    pub input_controller: InputController,
    #[component(include)]
    pub actor: Actor,
    #[reflect(hidden)]
    pub model_angle: SmoothAngle,
    #[reflect(description = "Pitch range for camera")]
    pitch_range: Range<f32>,
    #[reflect(hidden)]
    yaw: f32,
    #[reflect(hidden)]
    pitch: f32,
    #[reflect(hidden)]
    spectator_target: Handle<Node>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            model: Default::default(),
            camera: Default::default(),
            input_controller: Default::default(),
            actor: Default::default(),
            pitch_range: -90.0f32..90.0f32,
            yaw: 0.0,
            model_angle: SmoothAngle {
                angle: 0.0,
                target: 0.0,
                speed: 1.5 * std::f32::consts::TAU, // 540 deg/s
            },
            pitch: 0.0,
            spectator_target: Default::default(),
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
                camera.set_enabled(self.actor.kind == ActorKind::Player);
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

        if self.actor.kind == ActorKind::RemotePlayer
            || game
                .menu
                .as_ref()
                .map_or(false, |menu| menu.is_active(ctx.user_interface))
        {
            return;
        }

        // Spawn a ball on left mouse button click.
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::MouseInput { button, state, .. } = event {
                if *button == MouseButton::Left && *state == ElementState::Pressed {
                    if let Some(server) = game.server.as_mut() {
                        if let Ok(ball_prefab) = block_on(
                            ctx.resource_manager
                                .request::<Model>("data/models/cannon_ball.rgs"),
                        ) {
                            let rigid_body = &ctx.scene.graph[self.actor.rigid_body];
                            let forward_vec = rigid_body.look_vector();
                            let self_position = rigid_body.global_position();
                            server.broadcast_message_to_clients(ServerMessage::Instantiate(vec![
                                InstanceDescriptor {
                                    path: ball_prefab.kind().path().unwrap().to_path_buf(),
                                    position: self_position + forward_vec,
                                    rotation: Default::default(),
                                    velocity: Default::default(),
                                    ids: ball_prefab.generate_ids(),
                                },
                            ]));
                        }
                    }
                }
            }
        }

        let this = &ctx.scene.graph[ctx.handle];
        if self.input_controller.on_os_event(
            event,
            &self.pitch_range,
            ctx.dt,
            game.settings.read().mouse_sensitivity,
            game,
            &mut self.spectator_target,
        ) {
            if !game.level.leaderboard.is_finished(ctx.handle) {
                if let Some(client) = game.client.as_mut() {
                    client.send_message_to_server(ClientMessage::Input {
                        player: this.instance_id(),
                        input_state: self.input_controller.clone(),
                    })
                }
            }
        }
    }

    fn on_update(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get_mut::<Game>();

        if game.is_client() {
            return;
        }

        let finished = game.level.leaderboard.is_finished(ctx.handle);
        let response_speed = (1.0 - game.settings.read().mouse_smoothness).clamp(0.1, 1.0);
        self.pitch += (self.input_controller.target_pitch - self.pitch) * response_speed;
        self.yaw += (self.input_controller.target_yaw - self.yaw) * response_speed;

        let self_position = ctx.scene.graph[self.actor.rigid_body].global_position();
        let spectator_target_position = ctx
            .scene
            .graph
            .try_get_script_component_of::<Actor>(self.spectator_target)
            .and_then(|n| {
                ctx.scene
                    .graph
                    .try_get(n.rigid_body)
                    .map(|n| n.global_position())
            });

        if let Some(camera_controller) = ctx
            .scene
            .graph
            .try_get_script_component_of_mut::<CameraController>(self.camera)
        {
            camera_controller.pitch = self.pitch;
            camera_controller.yaw = self.yaw;
            if let (true, Some(spectator_target_position)) = (finished, spectator_target_position) {
                // Spectate a player.
                camera_controller.target_position = spectator_target_position;
            } else {
                camera_controller.target_position = self_position;
            }
        }

        let has_ground_contact = self.actor.has_ground_contact(&ctx.scene.graph);
        let is_in_jump_state = self.actor.is_in_jump_state(&ctx.scene.graph);

        self.actor.target_desired_velocity = Vector3::default();

        if let Some(rigid_body) = ctx.scene.graph[self.actor.rigid_body].cast_mut::<RigidBody>() {
            if !finished {
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
            }

            self.actor.target_desired_velocity = self
                .actor
                .target_desired_velocity
                .try_normalize(f32::EPSILON)
                .map(|v| v.scale(self.actor.speed))
                .unwrap_or_default();

            if !finished
                && self.input_controller.jump
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

            let is_moving = self.input_controller.move_left
                || self.input_controller.move_right
                || self.input_controller.move_forward
                || self.input_controller.move_backward;

            if is_moving {
                rigid_body
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        self.input_controller.target_yaw,
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
                    .set_scale(Vector3::repeat(0.01))
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
