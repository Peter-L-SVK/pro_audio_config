//! Main integration tests for the audio config tool

use pro_audio_config::{AudioSettings, detect_audio_device};

#[test]
fn test_library_compiles_and_works() {
    // Basic smoke test to ensure the library can be used
    let settings = AudioSettings::new(48000, 24, 512);
    assert_eq!(settings.sample_rate, 48000);
    assert_eq!(settings.bit_depth, 24);
    assert_eq!(settings.buffer_size, 512);
}

#[test]
fn test_multiple_audio_settings() {
    // Test various combinations of audio settings
    let test_cases = vec![
        (44100, 16, 256),
        (48000, 24, 512),
        (96000, 32, 1024),
        (192000, 24, 2048),
    ];

    for (sample_rate, bit_depth, buffer_size) in test_cases {
        let settings = AudioSettings::new(sample_rate, bit_depth, buffer_size);
        assert_eq!(settings.sample_rate, sample_rate);
        assert_eq!(settings.bit_depth, bit_depth);
        assert_eq!(settings.buffer_size, buffer_size);
    }
}

#[test]
fn test_detect_audio_device_integration() {
    // Integration test for device detection
    // This actually runs the system commands
    let result = detect_audio_device();
    
    // We can't guarantee success (depends on system state)
    // but we can verify the function runs and returns a Result
    match result {
        Ok(device_info) => {
            println!("Detected audio device: {}", device_info);
            assert!(!device_info.is_empty());
        }
        Err(e) => {
            println!("Device detection failed (expected in some environments): {}", e);
            // This is acceptable in test environments without audio devices
        }
    }
}
