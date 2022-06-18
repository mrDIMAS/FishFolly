//! Executor with your game connected to it as a plugin.
use fish_fall::Game;
use fyrox::{engine::executor::Executor, renderer::QualitySettings};

fn main() {
    let mut executor = Executor::new();
    executor.get_window().set_title("Fish Fall");
    let mut quality_settings = QualitySettings::default();
    quality_settings.use_ssao = false;
    executor
        .renderer
        .set_quality_settings(&quality_settings)
        .unwrap();
    executor.add_plugin(Game::new());
    executor.run()
}
