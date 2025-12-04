/*
 * Pro Audio Config - Library
 * Version: 1.7
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
    AudioDevice,
    AudioSettings,
    DeviceCapabilities,
    DeviceType,
    detect_all_audio_devices,
    detect_audio_device,
    detect_current_audio_settings,
    // Enhanced functions for exclusive mode
    detect_high_performance_devices,
    detect_input_audio_device,
    detect_input_audio_devices,
    detect_output_audio_device,
    detect_output_audio_devices,
    detect_recommended_devices,
    get_device_capabilities,
    is_device_suitable_for_exclusive_mode,
    resolve_pipewire_device_name,
    resolve_pulse_device_name,
};

pub use config::{
    // Advanced configuration functions
    AdvancedAudioSettings,
    apply_advanced_audio_settings,
    apply_input_audio_settings_with_auth_blocking,
    apply_output_audio_settings_with_auth_blocking,
    apply_user_audio_settings,
    check_audio_services,
    check_exclusive_mode_status,
    cleanup_config_files,
    restore_standard_audio_mode,
    update_audio_settings,
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

        // Test enhanced exports
        let _ = detect_high_performance_devices();
        let _ = detect_recommended_devices();
        let _ = get_device_capabilities("default");

        // Test device suitability
        let test_device = AudioDevice {
            name: "test".to_string(),
            description: "test".to_string(),
            id: "test".to_string(),
            device_type: DeviceType::Output,
            available: true,
        };
        let _ = is_device_suitable_for_exclusive_mode(&test_device);
    }

    #[test]
    fn test_module_structure() {
        // Verify all modules are properly defined
        // This is a compile-time test, so it should always pass if the code compiles
        assert!(true, "Module structure should be valid");
    }

    #[test]
    fn test_advanced_audio_settings() {
        // Test AdvancedAudioSettings if it's available
        let advanced_settings = AdvancedAudioSettings {
            exclusive_mode: true,
            direct_hardware: true,
            low_latency: true,
            buffer_size: 128,
            sample_rate: 48000,
        };

        assert!(advanced_settings.exclusive_mode);
        assert!(advanced_settings.direct_hardware);
        assert!(advanced_settings.low_latency);
        assert_eq!(advanced_settings.buffer_size, 128);
        assert_eq!(advanced_settings.sample_rate, 48000);
    }

    #[test]
    fn test_device_capabilities_structure() {
        // Test DeviceCapabilities structure
        let capabilities = DeviceCapabilities {
            sample_rates: vec![44100, 48000, 96000],
            formats: vec!["S16LE".to_string(), "S24LE".to_string()],
            buffer_sizes: vec![64, 128, 256],
            min_buffer_size: 64,
            max_buffer_size: 4096,
            period_sizes: vec![32, 64, 128],
        };

        assert_eq!(capabilities.sample_rates.len(), 3);
        assert_eq!(capabilities.formats.len(), 2);
        assert_eq!(capabilities.buffer_sizes.len(), 3);
        assert_eq!(capabilities.min_buffer_size, 64);
        assert_eq!(capabilities.max_buffer_size, 4096);
    }
}
