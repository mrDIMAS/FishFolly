//! Editor with your game connected to it as a plugin.

use fyroxed_base::{fyrox::event_loop::EventLoop, Editor, StartupData};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut editor = Editor::new(Some(StartupData {
        working_directory: Default::default(),
        scenes: vec!["data/maps/drake.rgs".into()],
    }));

    #[cfg(feature = "dylib")]
    editor
        .add_dynamic_plugin(
            // TODO: Windows-only
            "fish_fall_dylib.dll",
            true,
            true,
        )
        .unwrap();

    #[cfg(not(feature = "dylib"))]
    {
        use fish_fall::Game;
        editor.add_plugin(Game::new());
    }

    editor.run(event_loop)
}
