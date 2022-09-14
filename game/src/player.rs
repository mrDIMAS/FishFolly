//! Main player (host) script.

use crate::{game_mut, marker::Actor, utils, CameraController, Event};
use fyrox::{
    animation::machine::{Machine, Parameter},
    core::{
        algebra::{UnitQuaternion, Vector3},
        futures::executor::block_on,
        inspect::prelude::*,
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    event::{ElementState, VirtualKeyCode, WindowEvent},
    impl_component_provider,
    resource::absm::AbsmResource,
    scene::{
        graph::{map::NodeHandleMap, Graph},
        node::Node,
        node::TypeUuidProvider,
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

#[derive(Clone, Inspect, Visit, Debug, Reflect)]
pub struct Player {
    #[inspect(description = "Speed of the player.")]
    speed: f32,
    #[inspect(description = "Handle to player's collider.")]
    pub collider: Handle<Node>,
    #[inspect(description = "Animation blending state machine used by player's model.")]
    absm_resource: Option<AbsmResource>,
    #[inspect(description = "Handle to player's model.")]
    model: Handle<Node>,
    #[inspect(description = "Handle to a node with camera controller.")]
    #[visit(optional)]
    camera: Handle<Node>,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    absm: Handle<Machine>,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    pub input_controller: InputController,
    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    pub actor: Actor,
}

impl_component_provider!(Player, actor: Actor);

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 1.0,
            collider: Default::default(),
            absm_resource: None,
            model: Default::default(),
            camera: Default::default(),
            absm: Default::default(),
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
    fn on_init(&mut self, context: ScriptContext) {
        assert!(game_mut(context.plugins).actors.insert(context.handle));

        if self.model.is_some() {
            if let Some(absm_resource) = self.absm_resource.as_ref() {
                let animations =
                    block_on(absm_resource.load_animations(context.resource_manager.clone()));

                self.absm = absm_resource
                    .instantiate(self.model, context.scene, animations)
                    .unwrap();
            } else {
                Log::err("There is no resource specified for player ABSM!".to_owned());
            }
        } else {
            Log::err("There is no model set for player!".to_owned());
        }

        Log::info(format!("Player {:?} created!", context.handle));
    }

    fn on_deinit(&mut self, context: ScriptDeinitContext) {
        assert!(game_mut(context.plugins)
            .actors
            .remove(&context.node_handle));
        Log::info(format!("Player {:?} destroyed!", context.node_handle));
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: ScriptContext) {
        self.input_controller.on_os_event(event);
    }

    fn on_update(&mut self, context: ScriptContext) {
        let ScriptContext { handle, scene, .. } = context;

        let has_ground_contact = self.has_ground_contact(&scene.graph);

        let yaw = scene
            .graph
            .try_get(self.camera)
            .and_then(|c| c.script())
            .and_then(|s| s.cast::<CameraController>())
            .map(|c| c.yaw)
            .unwrap_or_default();

        if let Some(rigid_body) = scene.graph[handle].cast_mut::<RigidBody>() {
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

                scene.graph[self.model].local_transform_mut().set_rotation(
                    UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        (angle + 180.0).to_radians(),
                    ),
                );
            }

            scene.animation_machines[self.absm]
                .set_parameter("Run", Parameter::Rule(is_moving))
                .set_parameter("Jump", Parameter::Rule(jump));
        }
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        old_new_mapping
            .map(&mut self.model)
            .map(&mut self.collider)
            .map(&mut self.camera);
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        let mut state = resource_manager.state();
        let containers = state.containers_mut();
        containers
            .absm
            .try_restore_optional_resource(&mut self.absm_resource);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}
