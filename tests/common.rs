//! Common utilities for integration tests

use pro_audio_config::audio::{AudioDevice, AudioSettings, DeviceType};

/// Create test audio settings with valid defaults
pub fn create_test_settings() -> AudioSettings {
    AudioSettings::new(48000, 24, 512, "default".to_string())
}

/// Create test audio settings with specific values
pub fn create_custom_settings(
    sample_rate: u32,
    bit_depth: u32,
    buffer_size: u32,
    device_id: &str,
) -> AudioSettings {
    AudioSettings::new(sample_rate, bit_depth, buffer_size, device_id.to_string())
}

/// Create a test audio device for UI testing
pub fn create_test_audio_device(
    name: &str,
    description: &str,
    device_type: DeviceType,
) -> AudioDevice {
    AudioDevice {
        name: name.to_string(),
        description: description.to_string(),
        id: format!("alsa:{}", name.to_lowercase().replace(' ', "_")),
        device_type,
        available: true,
    }
}

/// Check if running in CI environment
pub fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok()
}

/// Should we run PipeWire integration tests that probe the system audio stack?
/// Default: false on CI (the workflow sets RUN_PIPEWIRE_INTEGRATION=false)
pub fn should_run_pipewire_integration() -> bool {
    std::env::var("RUN_PIPEWIRE_INTEGRATION").is_ok()
}

/// Simulate device categorization logic from UI
pub fn categorize_device(device: &AudioDevice) -> &'static str {
    let desc_lower = device.description.to_lowercase();
    let name_lower = device.name.to_lowercase();
    let id_lower = device.id.to_lowercase();

    if desc_lower.contains("usb") || name_lower.contains("usb") || id_lower.contains("usb") {
        "USB"
    } else if desc_lower.contains("hdmi")
        || name_lower.contains("hdmi")
        || desc_lower.contains("displayport")
        || name_lower.contains("displayport")
    {
        "HDMI"
    } else if name_lower.contains("pci") || id_lower.contains("pci") || desc_lower.contains("pci") {
        "PCI"
    } else {
        "Other"
    }
}
