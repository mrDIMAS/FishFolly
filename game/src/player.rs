use crate::{Event, Game};
use fyrox::{
    animation::machine::{Machine, Parameter},
    core::{
        algebra::{UnitQuaternion, Vector3},
        futures::executor::block_on,
        inspect::prelude::*,
        pool::Handle,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    event::{DeviceEvent, ElementState, VirtualKeyCode, WindowEvent},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
    resource::{absm::AbsmResource, model::Model},
    scene::{node::Node, node::TypeUuidProvider, rigidbody::RigidBody},
    script::{ScriptContext, ScriptTrait},
    utils::log::Log,
};

#[derive(Clone, Default, Debug)]
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
    pub fn update(&mut self, event: &Event<()>, dt: f32) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
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
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                self.yaw -= delta.0 as f32 * dt;
                self.pitch = (self.pitch + delta.1 as f32 * dt)
                    .clamp(-90.0f32.to_radians(), 90.0f32.to_radians());
            }
            _ => {}
        }
    }
}

#[derive(Clone, Inspect, Visit, Debug)]
pub struct Player {
    speed: f32,

    #[visit(optional)]
    model_pivot: Handle<Node>,

    #[visit(optional)]
    pub collider: Handle<Node>,

    #[visit(optional)]
    model_resource: Option<Model>,

    #[visit(optional)]
    absm_resource: Option<AbsmResource>,

    #[visit(skip)]
    #[inspect(skip)]
    model: Handle<Node>,

    #[visit(skip)]
    #[inspect(skip)]
    absm: Handle<Machine>,

    #[visit(skip)]
    #[inspect(skip)]
    pub input_controller: InputController,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 1.0,
            model_pivot: Default::default(),
            collider: Default::default(),
            model_resource: None,
            absm_resource: None,
            model: Default::default(),
            absm: Default::default(),
            input_controller: Default::default(),
        }
    }
}

impl TypeUuidProvider for Player {
    fn type_uuid() -> Uuid {
        uuid!("deb77c1d-668d-4716-a8f7-04ed09b0b9f6")
    }
}

impl ScriptTrait for Player {
    fn on_property_changed(&mut self, args: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, args,
            Self::SPEED => speed,
            Self::ABSM_RESOURCE => absm_resource,
            Self::MODEL_RESOURCE => model_resource,
            Self::MODEL_PIVOT => model_pivot,
            Self::COLLIDER => collider
        )
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        let mut state = resource_manager.state();
        let containers = state.containers_mut();
        containers
            .absm
            .try_restore_optional_resource(&mut self.absm_resource);
        containers
            .models
            .try_restore_optional_resource(&mut self.model_resource);
    }

    fn on_init(&mut self, context: ScriptContext) {
        if let Some(model_resource) = self.model_resource.as_ref() {
            // Wait until model is fully loaded.
            let _ = block_on(model_resource.clone());

            self.model = model_resource.instantiate_geometry(context.scene);

            context.scene.graph[self.model]
                .local_transform_mut()
                .set_scale(Vector3::new(0.01, 0.01, 0.01));

            context.scene.graph.link_nodes(
                self.model,
                if self.model_pivot.is_some() {
                    self.model_pivot
                } else {
                    context.handle
                },
            );

            if let Some(absm_resource) = self.absm_resource.as_ref() {
                self.absm = block_on(absm_resource.instantiate(
                    self.model,
                    context.scene,
                    context.resource_manager.clone(),
                ))
                .unwrap();
            } else {
                Log::err("There is no resource specified for player ABSM!".to_owned());
            }
        } else {
            Log::err("There is no resource specified for player model!".to_owned());
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, context: ScriptContext) {
        self.input_controller.update(event, context.dt);
    }

    fn on_update(&mut self, context: ScriptContext) {
        let ScriptContext { handle, scene, .. } = context;

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
            if self.input_controller.jump {
                velocity.y += 5.5;
                self.input_controller.jump = false;
                jump = true;
            }

            rigid_body.set_lin_vel(velocity);

            let is_moving = velocity.x != 0.0 || velocity.z != 0.0;

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

                scene.graph[self.model_pivot]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        (angle + 180.0).to_radians(),
                    ));
            }

            scene.animation_machines[self.absm]
                .set_parameter("Run", Parameter::Rule(is_moving))
                .set_parameter("Jump", Parameter::Rule(jump));
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn plugin_uuid(&self) -> Uuid {
        Game::type_uuid()
    }
}
