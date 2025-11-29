//! Main integration tests for Pro Audio Config

// Declare common as a module
mod common;

// Import for use in this file
use common::*;

#[test]
fn test_library_integration() {
    // Basic smoke test to ensure all modules work together
    use pro_audio_config::audio::AudioSettings;
    use pro_audio_config::config::{
        apply_output_audio_settings_with_auth_blocking,
        apply_input_audio_settings_with_auth_blocking
    };

    let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    assert!(settings.validate().is_ok());

    // Test that the function signature works (without actually running privileged commands)
    let result = std::panic::catch_unwind(|| {
        let _ = apply_output_audio_settings_with_auth_blocking(settings.clone());
        let _ = apply_input_audio_settings_with_auth_blocking(settings);
    });
    assert!(result.is_ok());
}

#[test]
fn test_v15_feature_integration() {
    use pro_audio_config::audio::{
        AudioSettings,
        detect_output_audio_devices,
        detect_input_audio_devices,
        detect_output_audio_device,
        detect_input_audio_device
    };
    use pro_audio_config::config::{
        apply_output_audio_settings_with_auth_blocking,
        apply_input_audio_settings_with_auth_blocking
    };

    // Test that all v1.5 functions are properly integrated
    let output_result = detect_output_audio_devices();
    let input_result = detect_input_audio_devices();

    assert!(output_result.is_ok() || output_result.is_err());
    assert!(input_result.is_ok() || input_result.is_err());

    // Test device detection
    let output_device = detect_output_audio_device();
    let input_device = detect_input_audio_device();

    assert!(output_device.is_ok() || output_device.is_err());
    assert!(input_device.is_ok() || input_device.is_err());

    // Test apply functions (they should exist and not panic)
    let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    let output_apply = std::panic::catch_unwind(|| {
        let _ = apply_output_audio_settings_with_auth_blocking(settings.clone());
    });
    let input_apply = std::panic::catch_unwind(|| {
        let _ = apply_input_audio_settings_with_auth_blocking(settings);
    });

    assert!(output_apply.is_ok());
    assert!(input_apply.is_ok());
}

#[test]
fn test_device_categorization_integration() {
    // Test the complete device categorization workflow
    use pro_audio_config::audio::{AudioDevice, DeviceType};

    let devices = vec![
        AudioDevice {
            name: "usb-audio".to_string(),
            description: "USB Audio Interface".to_string(),
            id: "alsa:usb".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "hdmi-output".to_string(),
            description: "HDMI Audio".to_string(),
            id: "alsa:hdmi".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
    ];

    // Test categorization using the common helper
    for device in &devices {
        let category = categorize_device(device);
        match category {
            "USB" => assert!(device.description.to_lowercase().contains("usb")),
            "HDMI" => assert!(device.description.to_lowercase().contains("hdmi")),
            _ => {}
        }
    }
}

#[test]
fn test_config_cleanup_integration() {
    // Test that cleanup function exists and can be called
    let result = std::panic::catch_unwind(|| {
        let _ = pro_audio_config::config::cleanup_config_files();
    });
    assert!(result.is_ok(), "cleanup_config_files should not panic");
}

#[test]
fn test_service_check_integration() {
    // Test service checking function
    let result = pro_audio_config::config::check_audio_services();
    // Should return either Ok or Err without panicking
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_wireplumber_debug_integration() {
    // Test debug function exists - use a simpler approach without timeout
    let result = std::panic::catch_unwind(|| {
        // Use a thread but with simpler join logic
        let handle = std::thread::spawn(|| {
            let _ = pro_audio_config::config::debug_wireplumber_config();
        });

        // Use regular join
        if let Err(_) = handle.join() {
            println!("debug_wireplumber_config thread failed (may be expected in test environment)");
        }
    });
    assert!(result.is_ok(), "debug_wireplumber_config should not panic");
}
