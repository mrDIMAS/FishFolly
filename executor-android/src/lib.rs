//! Android executor with your game connected to it as a plugin.
use fish_fall::Game;
use fyrox::{
    core::io, engine::executor::Executor, event_loop::EventLoopBuilder,
    platform::android::EventLoopBuilderExtAndroid,
};

#[no_mangle]
fn android_main(app: fyrox::platform::android::activity::AndroidApp) {
    io::ANDROID_APP
        .set(app.clone())
        .expect("ANDROID_APP cannot be set twice.");
    let event_loop = EventLoopBuilder::new()
        .with_android_app(app)
        .build()
        .unwrap();
    let mut executor = Executor::from_params(event_loop, Default::default());
    executor.add_plugin(Game::new());
    executor.run()
}
