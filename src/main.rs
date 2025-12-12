/*
 * Pro Audio Config
 * Version: 1.8
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Audio configuration tool for Linux PipeWire/ALSA systems (Jack in mind)
 */

use gtk::Application;
use gtk::prelude::*;
use pro_audio_config::AudioApp;

fn main() {
    let app = Application::new(Some("com.example.audio-config"), Default::default());

    app.connect_activate(|app| {
        let audio_app = AudioApp::new(app);
        audio_app.window.show_all();
    });

    app.run();
}
