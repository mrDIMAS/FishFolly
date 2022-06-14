//! Executor with your game connected to it as a plugin.
use fyrox::engine::executor::Executor;
use fish_fall::Game;

fn main() {
    let mut executor = Executor::new();
    executor.add_plugin(Game::new());
    executor.run()
}