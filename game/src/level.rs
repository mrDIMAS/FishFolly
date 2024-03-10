use fyrox::{
    core::pool::Handle,
    fxhash::FxHashMap,
    scene::{node::Node, Scene},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct LeaderBoardEntry {
    finished: bool,
    position: usize,
}

#[derive(Default)]
pub struct Leaderboard {
    entries: FxHashMap<Handle<Node>, LeaderBoardEntry>,
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
        entry.position = prev_position + 1;
        entry.finished = true;
    }
}

#[derive(Default)]
pub struct Level {
    pub scene: Handle<Scene>,
    pub targets: HashSet<Handle<Node>>,
    pub start_points: HashSet<Handle<Node>>,
    pub actors: HashSet<Handle<Node>>,
    pub respawners: HashSet<Handle<Node>>,
    pub leaderboard: Leaderboard,
}
