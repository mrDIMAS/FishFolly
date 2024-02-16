//! Executor runs the game in standalone (production) mode.
use fish_fall::GameConstructor;
use fyrox::engine::executor::Executor;
use fyrox::engine::GraphicsContextParams;
use fyrox::event_loop::EventLoop;

fn main() {
    let mut executor = Executor::from_params(
        EventLoop::new().unwrap(),
        GraphicsContextParams {
            window_attributes: Default::default(),
            vsync: false,
            msaa_sample_count: Some(4),
        },
    );

    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
