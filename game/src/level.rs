use crate::actor::Actor;
use fyrox::{
    core::{pool::Handle, visitor::prelude::*},
    fxhash::FxHashMap,
    graph::BaseSceneGraph,
    plugin::PluginContext,
    scene::{graph::Graph, node::Node, Scene},
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet, sync::mpsc::Sender};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Visit)]
pub struct LeaderBoardEntry {
    pub actor: Handle<Node>,
    pub finished: bool,
    pub real_time_position: usize,
    pub finished_position: usize,
}

pub enum LeaderBoardEvent {
    Finished { actor: Handle<Node>, place: usize },
}

#[derive(Default, Visit)]
pub struct Leaderboard {
    pub entries: FxHashMap<Handle<Node>, LeaderBoardEntry>,
    #[visit(skip)]
    pub sender: Option<Sender<LeaderBoardEvent>>,
    #[visit(skip)]
    temp_array: Vec<(Handle<Node>, f32)>,
}

impl Leaderboard {
    pub fn is_finished(&self, actor: Handle<Node>) -> bool {
        self.entries
            .get(&actor)
            .map(|e| e.finished)
            .unwrap_or_default()
    }

    pub fn finish(&mut self, actor: Handle<Node>) {
        let prev_position = self
            .entries
            .iter()
            .min_by_key(|(_, v)| v.finished_position)
            .map(|e| e.1.finished_position)
            .unwrap_or_default();
        let entry = self
            .entries
            .entry(actor)
            .or_insert_with(|| LeaderBoardEntry {
                actor,
                ..Default::default()
            });
        if !entry.finished {
            let place = prev_position + 1;
            entry.finished_position = place;
            entry.finished = true;
            if let Some(sender) = self.sender.as_ref() {
                sender
                    .send(LeaderBoardEvent::Finished { actor, place })
                    .unwrap();
            }
        }
    }

    pub fn update(
        &mut self,
        actors: &HashSet<Handle<Node>>,
        finish_point: Handle<Node>,
        graph: &Graph,
    ) {
        let Some(finish_point) = graph.try_get(finish_point).map(|n| n.global_position()) else {
            return;
        };

        self.temp_array.clear();
        for actor in actors {
            if let Some(actor_ref) = graph.try_get_script_component_of::<Actor>(*actor) {
                let position = graph[actor_ref.rigid_body].global_position();
                self.temp_array
                    .push((*actor, position.metric_distance(&finish_point)));
            }
        }

        self.temp_array
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

        for (position, (handle, _)) in self.temp_array.iter().enumerate() {
            let entry = self
                .entries
                .entry(*handle)
                .or_insert_with(|| LeaderBoardEntry {
                    actor: *handle,
                    ..Default::default()
                });
            entry.real_time_position = position;
        }
    }
}

#[derive(Visit)]
pub struct Level {
    pub scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
    pub respawners: HashSet<Handle<Node>>,
    pub leaderboard: Leaderboard,
    pub match_timer: f32,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            scene: Default::default(),
            targets: Default::default(),
            start_points: Default::default(),
            actors: Default::default(),
            respawners: Default::default(),
            leaderboard: Default::default(),
            match_timer: 15.0 * 60.0,
        }
    }
}

impl Level {
    pub fn update(&mut self, ctx: &PluginContext) {
        if let Some(scene) = ctx.scenes.try_get(self.scene) {
            self.match_timer = (self.match_timer - ctx.dt).max(0.0);

            self.leaderboard.update(
                &self.actors,
                self.targets.iter().next().cloned().unwrap_or_default(),
                &scene.graph,
            );
        }
    }

    pub fn sudden_death(&mut self) {
        if self.match_timer > 60.0 {
            self.match_timer = 60.0;
        }
    }

    pub fn is_time_critical(&self) -> bool {
        self.match_timer <= 60.0
    }

    pub fn is_match_ended(&self) -> bool {
        self.match_timer <= 0.0
    }
}
