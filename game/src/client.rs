use crate::menu::Menu;
use crate::{
    actor::Actor,
    level::Level,
    net::{ClientMessage, InstanceDescriptor, PlayerDescriptor, ServerMessage},
    Game,
};
use fyrox::graph::SceneGraph;
use fyrox::plugin::error::GameResult;
use fyrox::{
    core::{log::Log, net::NetStream, pool::Handle},
    plugin::PluginContext,
    resource::model::{Model, ModelResourceExtension},
    scene::{rigidbody::RigidBody, Scene},
};
use std::fmt::Formatter;
use std::{fmt::Debug, io, net::ToSocketAddrs};

pub struct FinishedPlayer {
    pub name: String,
    pub place: usize,
}

pub struct WinContext {
    pub timer: f32,
    pub players: Vec<FinishedPlayer>,
}

pub struct Client {
    connection: NetStream,
    pub win_context: Option<WinContext>,
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server")
    }
}

fn instantiate_objects(instances: Vec<InstanceDescriptor>, ctx: &mut PluginContext) {
    for new_instance in instances {
        ctx.task_pool.spawn_plugin_task(
            ctx.resource_manager.request::<Model>(&new_instance.path),
            move |result, game: &mut Game, ctx| {
                let scene = ctx.scenes.try_get_mut(game.level.scene)?;
                let instance = result?
                    .begin_instantiation(scene)
                    .with_position(new_instance.position)
                    .with_rotation(new_instance.rotation)
                    .with_ids(&new_instance.ids)
                    .finish();
                scene
                    .graph
                    .try_get_mut_of_type::<RigidBody>(instance)?
                    .set_lin_vel(new_instance.velocity);
                Ok(())
            },
        );
    }
}

fn add_players(players: Vec<PlayerDescriptor>, ctx: &mut PluginContext) {
    for player in players {
        ctx.task_pool.spawn_plugin_task(
            ctx.resource_manager.request::<Model>(&player.instance.path),
            move |result, game: &mut Game, ctx| {
                let scene = &mut ctx.scenes[game.level.scene];
                let root = result?
                    .begin_instantiation(scene)
                    .with_ids(&player.instance.ids)
                    .finish();
                let actor = scene.graph.try_get_script_component_of_mut::<Actor>(root)?;
                actor.kind = player.kind;
                let rigid_body = actor.rigid_body;
                scene
                    .graph
                    .try_get_mut(rigid_body)?
                    .local_transform_mut()
                    .set_position(player.instance.position);
                Ok(())
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
            win_context: None,
        })
    }

    pub fn send_message_to_server(&mut self, message: ClientMessage) {
        match self.connection.send_message(&message) {
            Ok(_) => {}
            Err(err) => Log::err(format!("Unable to send client message: {}", err)),
        }
    }

    pub fn read_messages(
        &mut self,
        level: &mut Level,
        menu: Option<&Menu>,
        ctx: &mut PluginContext,
    ) -> GameResult {
        while let Some(msg) = self.connection.pop_message() {
            match msg {
                ServerMessage::LoadLevel { path } => {
                    ctx.async_scene_loader.request(path);
                }
                ServerMessage::UpdateTick(data) => {
                    let scene = ctx.scenes.try_get_mut(level.scene)?;
                    for entry in data.nodes {
                        let (_, node) = scene.graph.node_by_id_mut(entry.node)?;
                        let transform = node.local_transform_mut();
                        if **transform.position() != entry.position {
                            transform.set_position(entry.position);
                        }
                        if **transform.rotation() != entry.rotation {
                            transform.set_rotation(entry.rotation);
                        }
                    }
                }
                ServerMessage::Instantiate(instances) => {
                    instantiate_objects(instances, ctx);
                }
                ServerMessage::AddPlayers(players) => add_players(players, ctx),
                ServerMessage::EndMatch => {
                    let scene = ctx.scenes.try_get(level.scene)?;
                    let mut players = level
                        .leaderboard
                        .entries
                        .values()
                        .map(|e| {
                            let actor = scene
                                .graph
                                .try_get_script_component_of::<Actor>(e.actor)
                                .unwrap();

                            FinishedPlayer {
                                name: actor.name.clone(),
                                place: e.finished_position,
                            }
                        })
                        .collect::<Vec<_>>();
                    players.sort_by_key(|e| e.place);

                    self.win_context = Some(WinContext {
                        timer: 10.0,
                        players,
                    });

                    if let Some(menu) = menu {
                        menu.set_menu_visibility(ctx.user_interfaces.first(), true);
                        menu.set_main_menu_visibility(ctx.user_interfaces.first(), true);
                    }

                    ctx.scenes.remove(level.scene);
                    level.scene = Handle::NONE;
                }
                ServerMessage::LeaderBoard(msg) => {
                    level.leaderboard.entries =
                        msg.players.into_iter().map(|e| (e.actor, e)).collect();
                }
            }
        }
        Ok(())
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(win_context) = self.win_context.as_mut() {
            win_context.timer -= dt;

            if win_context.timer <= 0.0 {
                self.win_context.take();
            }
        }
    }

    pub fn on_scene_loaded(
        &mut self,
        has_server: bool,
        scene: Handle<Scene>,
        ctx: &mut PluginContext,
    ) -> GameResult {
        let scene = ctx.scenes.try_get_mut(scene)?;
        if !has_server {
            scene.graph.physics.enabled.set_value_silent(false);
        }
        Ok(())
    }
}
