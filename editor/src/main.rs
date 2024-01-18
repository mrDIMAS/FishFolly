//! Editor with your game connected to it as a plugin.
use fish_fall::{actor::Actor, GameConstructor};
use fyrox::{
    event_loop::EventLoop,
    gui::inspector::editors::inspectable::InspectablePropertyEditorDefinition,
};
use fyroxed_base::{Editor, StartupData};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut editor = Editor::new(
        &event_loop,
        Some(StartupData {
            working_directory: Default::default(),
            scene: "data/drake.rgs".into(),
        }),
    );

    editor
        .inspector
        .property_editors
        .insert(InspectablePropertyEditorDefinition::<Actor>::new());
    editor.add_game_plugin(GameConstructor);
    editor.run(event_loop)
}
