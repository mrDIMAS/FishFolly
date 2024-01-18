//! Executor runs the game in standalone (production) mode.
use fish_fall::GameConstructor;
use fyrox::engine::executor::Executor;

fn main() {
    let mut executor = Executor::new();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
