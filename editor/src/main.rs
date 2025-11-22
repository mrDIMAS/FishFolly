//! Editor with your game connected to it as a plugin.

use fyroxed_base::{fyrox::event_loop::EventLoop, Editor, StartupData};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut editor = Editor::new(Some(StartupData {
        working_directory: Default::default(),
        scenes: vec!["data/maps/drake.rgs".into()],
        named_objects: false,
    }));

    #[cfg(feature = "dylib")]
    {
        #[cfg(target_os = "windows")]
        let file_name = "game_dylib.dll";
        #[cfg(target_os = "linux")]
        let file_name = "libgame_dylib.so";
        #[cfg(target_os = "macos")]
        let file_name = "libgame_dylib.dylib";
        editor.add_dynamic_plugin(file_name, true, true).unwrap();
    }

    #[cfg(not(feature = "dylib"))]
    {
        use fish_fall::Game;
        editor.add_game_plugin(Game::new());
    }

    editor.run(event_loop)
}
