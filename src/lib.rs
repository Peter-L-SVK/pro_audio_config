/*
 * Pro Audio Config - Library
 * Version: 1.6
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Module library for app
 */

pub mod audio;
pub mod config;
pub mod ui;
pub mod utils;

// Re-export main functionality
pub use audio::{
    AudioSettings, AudioDevice, DeviceType,
    detect_audio_device, detect_all_audio_devices, detect_current_audio_settings,
    detect_output_audio_devices, detect_input_audio_devices,
    detect_output_audio_device, detect_input_audio_device,
    resolve_pipewire_device_name, resolve_pulse_device_name
};

pub use config::{
    apply_output_audio_settings_with_auth_blocking,
    apply_input_audio_settings_with_auth_blocking,
    apply_user_audio_settings,
    update_audio_settings,
    cleanup_config_files,
    check_audio_services
};

pub use ui::{AudioApp, create_section_box, show_error_dialog, show_success_dialog};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[cfg(test)]
    pub mod test_utils {

	// Re-export test helpers
    }

    #[test]
    fn test_library_exports() {
        // Test that all main functionality is properly exported
        let _ = AudioSettings::new(48000, 24, 512, "default".to_string());
        let _ = detect_audio_device();
        let _ = detect_all_audio_devices();
        let _ = detect_current_audio_settings();

        // Test new exports
        let _ = detect_output_audio_devices();
        let _ = detect_input_audio_devices();
        let _ = detect_output_audio_device();
        let _ = detect_input_audio_device();
    }

    #[test]
    fn test_module_structure() {
        // Verify all modules are properly defined
        // This is a compile-time test, so it should always pass if the code compiles
        assert!(true, "Module structure should be valid");
    }
}
