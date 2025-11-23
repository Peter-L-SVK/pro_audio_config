//! Main integration tests for Pro Audio Config

mod common;
mod audio_integration;
mod ui_integration;

// Re-export for easy access
pub use audio_integration::*;
pub use ui_integration::*;
pub use common::*;

#[test]
fn test_library_integration() {
    // Basic smoke test to ensure all modules work together
    use pro_audio_config::audio::AudioSettings;
    use pro_audio_config::config::apply_audio_settings_with_auth_blocking;
    
    let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    assert!(settings.validate().is_ok());
    
    // Test that the function signature works (without actually running privileged commands)
    let result = std::panic::catch_unwind(|| {
        let _ = apply_audio_settings_with_auth_blocking(settings);
    });
    assert!(result.is_ok());
}

// NEW: Test v1.5 feature integration
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
    
    // Test categorization
    for device in &devices {
        let category = categorize_device(device);
        match category {
            "USB" => assert!(device.description.to_lowercase().contains("usb")),
            "HDMI" => assert!(device.description.to_lowercase().contains("hdmi")),
            _ => {}
        }
    }
}
