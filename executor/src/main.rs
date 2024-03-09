//! Executor runs the game in standalone (production) mode.
use fish_fall::GameConstructor;
use fyrox::{
    dpi::LogicalSize, engine::executor::Executor, engine::GraphicsContextParams,
    event_loop::EventLoop, window::WindowAttributes,
};

fn main() {
    let mut window_attributes = WindowAttributes::default();
    window_attributes.inner_size = Some(LogicalSize::new(1366.0, 768.0).into());
    window_attributes.title = "Fish Folly".to_string();

    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes,
            vsync: false,
            msaa_sample_count: Some(4),
        },
    );

    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
