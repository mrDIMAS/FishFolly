//! Executor runs the game in standalone (production) mode.
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

    #[cfg(feature = "dylib")]
    {
        #[cfg(target_os = "windows")]
        let file_name = "game_dylib.dll";
        #[cfg(target_os = "linux")]
        let file_name = "libgame_dylib.so";
        #[cfg(target_os = "macos")]
        let file_name = "libgame_dylib.dylib";
        executor.add_dynamic_plugin(file_name, true, true).unwrap();
    }

    #[cfg(not(feature = "dylib"))]
    {
        use fish_fall::Game;
        executor.add_plugin(Game::new());
    }

    executor.run()
}
