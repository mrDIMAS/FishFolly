//! Executor runs the game in standalone (production) mode.
use fish_fall::GameConstructor;
use fyrox::{engine::executor::Executor, renderer::QualitySettings};

fn main() {
    let mut executor = Executor::new();
    executor.get_window().set_title("Fish Folly");
    let mut quality_settings = QualitySettings::default();
    quality_settings.use_ssao = false;
    executor
        .renderer
        .set_quality_settings(&quality_settings)
        .unwrap();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
