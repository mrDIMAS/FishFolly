use fyrox::{
    core::pool::Handle,
    fxhash::FxHashMap,
    scene::{node::Node, Scene},
};
use std::{collections::HashSet, sync::mpsc::Sender};

#[derive(Default)]
pub struct LeaderBoardEntry {
    finished: bool,
    position: usize,
}

pub enum LeaderBoardEvent {
    Finished { actor: Handle<Node>, place: usize },
}

#[derive(Default)]
pub struct Leaderboard {
    entries: FxHashMap<Handle<Node>, LeaderBoardEntry>,
    pub sender: Option<Sender<LeaderBoardEvent>>,
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
            .min_by_key(|(_, v)| v.position)
            .map(|e| e.1.position)
            .unwrap_or_default();
        let entry = self.entries.entry(actor).or_default();
        if !entry.finished {
            let place = prev_position + 1;
            entry.position = place;
            entry.finished = true;
            if let Some(sender) = self.sender.as_ref() {
                sender
                    .send(LeaderBoardEvent::Finished { actor, place })
                    .unwrap();
            }
        }
    }
}

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
    pub fn update(&mut self, dt: f32) {
        if self.scene.is_some() {
            self.match_timer = (self.match_timer - dt).max(0.0);
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
