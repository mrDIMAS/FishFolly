//! Editor with your game connected to it as a plugin.
use fish_fall::Game;
use fyrox::event_loop::EventLoop;
use fyroxed_base::{Editor, StartupData};

fn main() {
    let event_loop = EventLoop::new();
    let mut editor = Editor::new(
        &event_loop,
        Some(StartupData {
            working_directory: Default::default(),
            scene: "data/scene.rgs".into(),
        }),
    );

    editor.add_game_plugin(Game::default());
    editor.run(event_loop)
}
