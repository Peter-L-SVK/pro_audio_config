use pro_audio_config::AudioApp;
use gtk::prelude::*;
use gtk::Application;

fn main() {
    let app = Application::new(
        Some("com.example.audio-config"),
        Default::default(),
    );

    app.connect_activate(|app| {
        let audio_app = AudioApp::new(app);
        audio_app.window.show_all();
    });

    app.run();
}
