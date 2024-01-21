use crate::{
    actor::Actor,
    net::{ClientMessage, InstanceDescriptor, NetStream, PlayerDescriptor, ServerMessage},
    Game,
};
use fyrox::{
    core::log::Log,
    plugin::PluginContext,
    resource::model::{Model, ModelResourceExtension},
    scene::rigidbody::RigidBody,
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
                    let instance =
                        model.instantiate_at(scene, new_instance.position, new_instance.rotation);

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
            ctx.resource_manager.request::<Model>(&player.path),
            move |result, game: &mut Game, ctx| match result {
                Ok(model) => {
                    let scene = &mut ctx.scenes[game.scene];
                    let root = model.instantiate(scene);
                    if let Some(actor) = scene.graph.try_get_script_component_of::<Actor>(root) {
                        let rigid_body = actor.rigid_body;
                        if let Some(rigid_body) = scene.graph.try_get_mut(rigid_body) {
                            rigid_body
                                .local_transform_mut()
                                .set_position(player.position);
                        }
                    }
                }
                Err(err) => {
                    Log::err(format!(
                        "Unable to instantiate {} prefab. Reason: {:?}",
                        player.path.display(),
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

    pub fn send_message(&mut self, message: ClientMessage) {
        match self.connection.send_message(&message) {
            Ok(_) => {}
            Err(err) => Log::err(format!("Unable to send client message: {}", err)),
        }
    }

    pub fn read_messages(&mut self, ctx: &mut PluginContext) {
        self.connection.process_input(|msg| match msg {
            ServerMessage::LoadLevel { path } => {
                ctx.async_scene_loader.request(path);
            }
            ServerMessage::UpdateTick(data) => {}
            ServerMessage::Instantiate(instances) => {
                instantiate_objects(instances, ctx);
            }
            ServerMessage::AddPlayers(players) => add_players(players, ctx),
        })
    }
}
