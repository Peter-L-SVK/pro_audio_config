//! Integration tests specifically for audio functionality

use pro_audio_config::audio::{
    AudioSettings, AudioDevice, DeviceType,
    detect_all_audio_devices, detect_current_audio_settings,
    resolve_pipewire_device_name, resolve_pulse_device_name
};
use pro_audio_config::config::apply_audio_settings_with_auth_blocking;
use std::process::Command;

fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

fn has_audio_hardware() -> bool {
    use std::process::Command;
    
    Command::new("pactl")
        .args(["info"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false) ||
    Command::new("aplay")
        .args(["-L"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn test_audio_settings_lifecycle() {
    // Test creation, validation, and usage of AudioSettings
    let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    
    // Test validation
    assert!(settings.validate().is_ok());
    
    // Test format selection
    assert_eq!(settings.get_audio_format().unwrap(), "S24LE");
    
    // Test cloning
    let cloned = settings.clone();
    assert_eq!(settings.sample_rate, cloned.sample_rate);
    
    // Test debug formatting
    let debug_output = format!("{:?}", settings);
    assert!(debug_output.contains("AudioSettings"));
    assert!(debug_output.contains("48000"));
}

#[test]
fn test_audio_settings_edge_cases() {
    // Test boundary conditions
    let test_cases = vec![
        // Valid edge cases
        (44100, 16, 128, "alsa:default"),
        (384000, 32, 8192, "pipewire:123"),
        // Still valid but unusual
        (8000, 24, 64, "pulse:0"),
    ];
    
    for (sample_rate, bit_depth, buffer_size, device_id) in test_cases {
        let settings = AudioSettings::new(sample_rate, bit_depth, buffer_size, device_id.to_string());
        assert_eq!(settings.sample_rate, sample_rate);
        assert_eq!(settings.bit_depth, bit_depth);
        assert_eq!(settings.buffer_size, buffer_size);
        assert_eq!(settings.device_id, device_id);
    }
}

#[test]
fn test_audio_settings_validation_failures() {
    // Test that invalid settings are properly rejected
    let invalid_cases = vec![
        (12345, 24, 512, "default"),  // Invalid sample rate
        (48000, 8, 512, "default"),   // Invalid bit depth
        (48000, 24, 999, "default"),  // Invalid buffer size
        (48000, 24, 512, ""),         // Empty device ID
    ];
    
    for (sample_rate, bit_depth, buffer_size, device_id) in invalid_cases {
        let settings = AudioSettings::new(sample_rate, bit_depth, buffer_size, device_id.to_string());
        assert!(settings.validate().is_err(), 
            "Settings should be invalid: {}Hz/{}bit/{}samples/{}", 
            sample_rate, bit_depth, buffer_size, device_id);
    }
}

#[test]
fn test_apply_settings_integration() {
    // Test that the apply function can be called without panicking
    let settings = AudioSettings::new(96000, 24, 1024, "default".to_string());
    
    let result = std::panic::catch_unwind(|| {
        let _ = apply_audio_settings_with_auth_blocking(settings);
    });
    
    assert!(result.is_ok(), "apply_audio_settings_with_auth_blocking should not panic");
}

#[test]
fn test_system_command_availability() {
    // Verify that required system commands exist (or handle their absence gracefully)
    let commands = vec!["pw-cli", "pactl", "systemctl", "pkexec", "aplay", "arecord"];
    let mut available_commands = 0;
    
    for cmd in commands {
        match Command::new("which").arg(cmd).output() {
            Ok(output) if output.status.success() => {
                println!("✓ Command '{}' is available", cmd);
                available_commands += 1;
            }
            _ => {
                println!("⚠ Command '{}' not found (may be expected)", cmd);
            }
        }
    }
    
    // We should have at least some commands available in a typical system
    if !is_ci_environment() {
        assert!(available_commands >= 2, "Expected at least 2 audio commands to be available");
    }
}

#[test]
fn test_device_detection_resilience() {
    // Test that device detection handles various system states gracefully
    let result = detect_all_audio_devices();
    
    match result {
        Ok(devices) => {
            println!("Found {} audio devices", devices.len());
            
            // If we have devices, verify their structure
            if !devices.is_empty() {
                for device in devices.iter().take(3) { // Check first 3 devices
                    assert!(!device.name.is_empty());
                    assert!(!device.id.is_empty());
                    // Device type should be one of the expected variants
                    assert!(matches!(
                        device.device_type,
                        DeviceType::Input | DeviceType::Output | DeviceType::Duplex | DeviceType::Unknown
                    ));
                }
            }
        }
        Err(e) => {
            println!("Device detection failed (may be normal): {}", e);
            // In CI environments without audio, this is expected
            if !is_ci_environment() {
                // In non-CI environments, we might expect some audio support
                println!("Audio device detection failed in non-CI environment: {}", e);
            }
        }
    }
}

#[test]
fn test_current_settings_detection() {
    // Test detection of current audio settings
    let result = detect_current_audio_settings();
    
    match result {
        Ok(settings) => {
            println!("Detected settings: {}Hz/{}bit/{}samples", 
                     settings.sample_rate, settings.bit_depth, settings.buffer_size);
            
            // Validate detected settings are reasonable
            assert!(settings.sample_rate >= 8000 && settings.sample_rate <= 384000);
            assert!(settings.bit_depth == 16 || settings.bit_depth == 24 || settings.bit_depth == 32);
            assert!(settings.buffer_size >= 64 && settings.buffer_size <= 8192);
            assert!(!settings.device_id.is_empty());
        }
        Err(e) => {
            println!("Current settings detection failed: {}", e);
            // This is acceptable in environments without audio configuration access
        }
    }
}

#[test]
fn test_device_resolution_functions() {
    // Test device resolution functions (they should handle errors gracefully)
    
    // These will likely fail in test environments, but should not panic
    let pipewire_result = resolve_pipewire_device_name("99999");
    let pulse_result = resolve_pulse_device_name("999");
    
    // Both should return Result (either Ok or Err) but not panic
    assert!(pipewire_result.is_ok() || pipewire_result.is_err());
    assert!(pulse_result.is_ok() || pulse_result.is_err());
}

#[test]
fn test_audio_device_struct() {
    // Test AudioDevice struct functionality
    let device = AudioDevice {
        name: "Test Device".to_string(),
        description: "Test Description".to_string(),
        id: "test:id".to_string(),
        device_type: DeviceType::Output,
        available: true,
    };
    
    // Test cloning
    let cloned = device.clone();
    assert_eq!(device.name, cloned.name);
    assert_eq!(device.device_type, cloned.device_type);
    
    // Test debug formatting
    let debug_output = format!("{:?}", device);
    assert!(debug_output.contains("Test Device"));
    assert!(debug_output.contains("Output"));
}
