/*
 * Pro Audio Config Library
 * Version: 1.0
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Audio configuration tool for Linux PipeWire/ALSA systems (Jack in mind)
 */

pub mod audio;
pub mod ui;
pub mod utils;

pub use audio::{
    AudioSettings, 
    apply_audio_settings_with_auth_blocking, 
    detect_audio_device, 
    detect_current_audio_settings
};
pub use ui::{AudioApp, create_section_box, show_error_dialog, show_success_dialog};
