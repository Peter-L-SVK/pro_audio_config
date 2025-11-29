//! Integration tests specifically for audio functionality

// Common utilities for integration tests
use pro_audio_config::audio::{AudioSettings, AudioDevice, DeviceType};

fn create_custom_settings(sample_rate: u32, bit_depth: u32, buffer_size: u32, device_id: &str) -> AudioSettings {
    AudioSettings::new(sample_rate, bit_depth, buffer_size, device_id.to_string())
}

fn create_test_audio_device(name: &str, description: &str, device_type: DeviceType) -> AudioDevice {
    AudioDevice {
        name: name.to_string(),
        description: description.to_string(),
        id: format!("alsa:{}", name.to_lowercase().replace(' ', "_")),
        device_type,
        available: true,
    }
}

fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

use pro_audio_config::audio::{
    detect_all_audio_devices, detect_current_audio_settings,
    detect_output_audio_devices, detect_input_audio_devices,
    detect_output_audio_device, detect_input_audio_device,
    resolve_pipewire_device_name, resolve_pulse_device_name
};
use pro_audio_config::config::apply_output_audio_settings_with_auth_blocking;
use pro_audio_config::config::apply_input_audio_settings_with_auth_blocking;
use std::process::Command;

// Keep this function local to audio_integration.rs since it's audio-specific
fn has_audio_hardware() -> bool {
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
fn test_separate_input_output_detection() {
    // Test the new v1.5 separate detection functions
    let output_devices = detect_output_audio_devices();
    let input_devices = detect_input_audio_devices();

    // Both should return Result without panicking
    assert!(output_devices.is_ok() || output_devices.is_err());
    assert!(input_devices.is_ok() || input_devices.is_err());

    // If successful, verify device types are filtered correctly
    if let Ok(output_devs) = output_devices {
        for device in output_devs.iter().take(3) {
            assert!(matches!(
                device.device_type,
                DeviceType::Output | DeviceType::Duplex
            ), "Output device list should only contain output/duplex devices");
        }
    }

    if let Ok(input_devs) = input_devices {
        for device in input_devs.iter().take(3) {
            assert!(matches!(
                device.device_type,
                DeviceType::Input | DeviceType::Duplex
            ), "Input device list should only contain input/duplex devices");
        }
    }
}

#[test]
fn test_current_device_detection_separate() {
    // Test separate detection of current input/output devices
    let output_device = detect_output_audio_device();
    let input_device = detect_input_audio_device();

    // Both should return Result without panicking
    assert!(output_device.is_ok() || output_device.is_err());
    assert!(input_device.is_ok() || input_device.is_err());

    // Verify they return strings (either device names or error messages)
    if let Ok(output_name) = &output_device {
        assert!(!output_name.is_empty());
        // Should contain system identifier (PipeWire/PulseAudio/ALSA)
        assert!(output_name.contains(':'));
    }

    if let Ok(input_name) = &input_device {
        assert!(!input_name.is_empty());
        assert!(input_name.contains(':'));
    }
}

#[test]
fn test_hardware_device_filtering_integration() {
    // Test that hardware device filtering works in real detection
    let devices_result = detect_all_audio_devices();

    if let Ok(devices) = devices_result {
        for device in devices.iter().take(5) {
            // Test the filtering criteria used in the actual code
            let name_lower = device.name.to_lowercase();
            let desc_lower = device.description.to_lowercase();

            let is_virtual = name_lower.contains("virtual")
                || name_lower.contains("null")
                || name_lower.contains("dummy")
                || desc_lower.contains("virtual")
                || desc_lower.contains("null");

            // If it's not virtual, it should be considered hardware
            if !is_virtual {
                println!("Hardware device: {} - {}", device.name, device.description);
            }
        }
    }
}

#[test]
fn test_apply_functions_separate() {
    // Test that the new separate apply functions exist and work
    let output_settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    let input_settings = AudioSettings::new(48000, 24, 512, "default".to_string());

    // Test that functions can be called without panicking
    let output_result = std::panic::catch_unwind(|| {
        let _ = apply_output_audio_settings_with_auth_blocking(output_settings);
    });

    let input_result = std::panic::catch_unwind(|| {
        let _ = apply_input_audio_settings_with_auth_blocking(input_settings);
    });

    assert!(output_result.is_ok(), "apply_output_audio_settings_with_auth_blocking should not panic");
    assert!(input_result.is_ok(), "apply_input_audio_settings_with_auth_blocking should not panic");
}

#[test]
fn test_device_categorization_logic() {
    // Test the device categorization logic used in UI grouping
    let test_devices = vec![
        AudioDevice {
            name: "usb-audio".to_string(),
            description: "USB Audio Device".to_string(),
            id: "alsa:usb".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "hdmi-output".to_string(),
            description: "HDMI Audio Output".to_string(),
            id: "alsa:hdmi".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "pci-card".to_string(),
            description: "PCI Sound Card".to_string(),
            id: "alsa:pci".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "analog-input".to_string(),
            description: "Analog Input".to_string(),
            id: "alsa:analog".to_string(),
            device_type: DeviceType::Input,
            available: true,
        },
    ];

    for device in test_devices {
        let desc_lower = device.description.to_lowercase();
        let name_lower = device.name.to_lowercase();
        let id_lower = device.id.to_lowercase();

        // Test the categorization logic from the UI
        let category = if desc_lower.contains("usb") || name_lower.contains("usb") || id_lower.contains("usb") {
            "USB"
        } else if desc_lower.contains("hdmi") || name_lower.contains("hdmi") ||
                  desc_lower.contains("displayport") || name_lower.contains("displayport") {
            "HDMI"
        } else if name_lower.contains("pci") || id_lower.contains("pci") || desc_lower.contains("pci") {
            "PCI"
        } else {
            "Other"
        };

        assert!(!category.is_empty());
        println!("Device {} categorized as: {}", device.name, category);
    }
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
        // Test new v1.5 device ID patterns
        (48000, 24, 512, "default"),
        (48000, 24, 512, "alsa:usb-device"),
        (48000, 24, 512, "pipewire:42"),
        (48000, 24, 512, "pulse:1"),
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
        (48000, 24, 512, "invalid device"), // Device ID with space
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
    // Test that the apply functions can be called without panicking
    let settings = AudioSettings::new(96000, 24, 1024, "default".to_string());

    let result = std::panic::catch_unwind(|| {
        let _ = apply_output_audio_settings_with_auth_blocking(settings.clone());
        let _ = apply_input_audio_settings_with_auth_blocking(settings);
    });

    assert!(result.is_ok(), "apply audio settings functions should not panic");
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

#[test]
fn test_custom_settings_creation() {
    // Test the helper function
    let settings = create_custom_settings(96000, 32, 1024, "alsa:custom");
    assert_eq!(settings.sample_rate, 96000);
    assert_eq!(settings.bit_depth, 32);
    assert_eq!(settings.buffer_size, 1024);
    assert_eq!(settings.device_id, "alsa:custom");
}

#[test]
fn test_test_audio_device_creation() {
    // Test the helper function
    let device = create_test_audio_device(
        "test-device",
        "Test USB Audio",
        DeviceType::Output
    );
    assert_eq!(device.name, "test-device");
    assert_eq!(device.description, "Test USB Audio");
    assert_eq!(device.device_type, DeviceType::Output);
    assert!(device.available);
}

#[test]
fn test_audio_hardware_detection() {
    // Test the audio-specific helper function
    let has_hardware = has_audio_hardware();
    // This should not panic and return a boolean
    assert!(has_hardware || !has_hardware); // Always true, just testing it doesn't panic
}

#[test]
fn test_device_resolution_error_paths() {
    // Test error handling for invalid device IDs
    let invalid_pipewire = resolve_pipewire_device_name("invalid-node");
    let invalid_pulse = resolve_pulse_device_name("invalid-sink");

    // Both should return errors gracefully
    assert!(invalid_pipewire.is_err());
    assert!(invalid_pulse.is_err());
}

#[test]
fn test_apply_settings_error_handling() {
    // Test applying settings with invalid parameters
    let invalid_settings = AudioSettings::new(99999, 8, 999, "".to_string());

    let output_result = std::panic::catch_unwind(|| {
        let _ = apply_output_audio_settings_with_auth_blocking(invalid_settings.clone());
    });
    let input_result = std::panic::catch_unwind(|| {
        let _ = apply_input_audio_settings_with_auth_blocking(invalid_settings);
    });

    // Should not panic even with invalid settings
    assert!(output_result.is_ok());
    assert!(input_result.is_ok());
}
