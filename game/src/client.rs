use crate::{
    actor::Actor,
    net::{ClientMessage, InstanceDescriptor, PlayerDescriptor, ServerMessage},
    Game,
};
use fyrox::{
    core::net::NetStream,
    core::{log::Log, pool::Handle},
    plugin::PluginContext,
    resource::model::{Model, ModelResourceExtension},
    scene::{rigidbody::RigidBody, Scene},
};
use std::{fmt::Debug, io, net::ToSocketAddrs};

pub struct Client {
    connection: NetStream,
}

fn instantiate_objects(instances: Vec<InstanceDescriptor>, ctx: &mut PluginContext) {
    for new_instance in instances {
        ctx.task_pool.spawn_plugin_task(
            ctx.resource_manager.request::<Model>(&new_instance.path),
            move |result, game: &mut Game, ctx| match result {
                Ok(model) => {
                    let scene = &mut ctx.scenes[game.scene];
                    let instance = model
                        .begin_instantiation(scene)
                        .with_position(new_instance.position)
                        .with_rotation(new_instance.rotation)
                        .with_ids(&new_instance.ids)
                        .finish();

                    if let Some(rigid_body) = scene.graph[instance].cast_mut::<RigidBody>() {
                        rigid_body.set_lin_vel(new_instance.velocity);
                    }
                }
                Err(err) => {
                    Log::err(format!(
                        "Unable to instantiate {} prefab. Reason: {:?}",
                        new_instance.path.display(),
                        err
                    ));
                }
            },
        );
    }
}

fn add_players(players: Vec<PlayerDescriptor>, ctx: &mut PluginContext) {
    for player in players {
        ctx.task_pool.spawn_plugin_task(
            ctx.resource_manager.request::<Model>(&player.instance.path),
            move |result, game: &mut Game, ctx| match result {
                Ok(model) => {
                    let scene = &mut ctx.scenes[game.scene];
                    let root = model
                        .begin_instantiation(scene)
                        .with_ids(&player.instance.ids)
                        .finish();
                    if let Some(actor) = scene.graph.try_get_script_component_of_mut::<Actor>(root)
                    {
                        actor.is_remote = player.is_remote;
                        let rigid_body = actor.rigid_body;
                        if let Some(rigid_body) = scene.graph.try_get_mut(rigid_body) {
                            rigid_body
                                .local_transform_mut()
                                .set_position(player.instance.position);
                        }
                    }
                }
                Err(err) => {
                    Log::err(format!(
                        "Unable to instantiate {} prefab. Reason: {:?}",
                        player.instance.path.display(),
                        err
                    ));
                }
            },
        );
    }
}

impl Client {
    pub fn try_connect<A>(server_addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs + Debug,
    {
        Ok(Self {
            connection: NetStream::connect(server_addr)?,
        })
    }

    pub fn send_message_to_server(&mut self, message: ClientMessage) {
        match self.connection.send_message(&message) {
            Ok(_) => {}
            Err(err) => Log::err(format!("Unable to send client message: {}", err)),
        }
    }

    pub fn read_messages(&mut self, scene: Handle<Scene>, ctx: &mut PluginContext) {
        self.connection.process_input(|msg| match msg {
            ServerMessage::LoadLevel { path } => {
                ctx.async_scene_loader.request(path);
            }
            ServerMessage::UpdateTick(data) => {
                if let Some(scene) = ctx.scenes.try_get_mut(scene) {
                    for entry in data.nodes {
                        if let Some((_, node)) = scene.graph.node_by_id_mut(entry.node) {
                            let transform = node.local_transform_mut();
                            if **transform.position() != entry.position {
                                transform.set_position(entry.position);
                            }
                            if **transform.rotation() != entry.rotation {
                                transform.set_rotation(entry.rotation);
                            }
                        }
                    }
                }
            }
            ServerMessage::Instantiate(instances) => {
                instantiate_objects(instances, ctx);
            }
            ServerMessage::AddPlayers(players) => add_players(players, ctx),
        })
    }

    pub fn on_scene_loaded(
        &mut self,
        has_server: bool,
        scene: Handle<Scene>,
        ctx: &mut PluginContext,
    ) {
        let scene = &mut ctx.scenes[scene];
        if !has_server {
            scene.graph.physics.enabled.set_value_silent(false);
        }
    }
}
