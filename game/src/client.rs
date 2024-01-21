use crate::{
    actor::Actor,
    net::{ClientMessage, InstanceDescriptor, NetStream, PlayerDescriptor, ServerMessage},
    Game,
};
use fyrox::{
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
                    if let Some(actor) = scene.graph.try_get_script_component_of_mut::<Actor>(root)
                    {
                        actor.is_remote = player.is_remote;
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
                        if let Some(node) = scene.graph.try_get_mut(entry.node) {
                            if let Some(rigid_body) = node.query_component_mut::<RigidBody>() {
                                if rigid_body.lin_vel() != entry.velocity {
                                    rigid_body.set_lin_vel(entry.velocity);
                                }
                                if rigid_body.ang_vel() != entry.angular_velocity {
                                    rigid_body.set_ang_vel(entry.angular_velocity);
                                }
                            }

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
}
