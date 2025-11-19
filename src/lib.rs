/*
 * Pro Audio Config Library
 * Version: 1.2
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 */

pub mod audio;
pub mod config;
pub mod ui;
pub mod utils;

// Re-export main functionality
pub use audio::{
    AudioSettings, AudioDevice, DeviceType,
    detect_audio_device, detect_all_audio_devices, detect_current_audio_settings,
    resolve_pipewire_device_name, resolve_pulse_device_name
};
pub use config::apply_audio_settings_with_auth_blocking;
pub use ui::{AudioApp, create_section_box, show_error_dialog, show_success_dialog};
