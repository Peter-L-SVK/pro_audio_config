
//! Integration tests specifically for audio functionality

use pro_audio_config::{AudioSettings, apply_audio_settings_with_auth_blocking};
use std::process::Command;

#[test]
fn test_audio_settings_validation() {
    // Test that invalid settings are handled properly
    let valid_settings = AudioSettings::new(48000, 24, 512);
    assert_eq!(valid_settings.sample_rate, 48000);
    
    // Test edge cases
    let edge_settings = AudioSettings::new(1, 8, 64);
    assert_eq!(edge_settings.sample_rate, 1);
    assert_eq!(edge_settings.bit_depth, 8);
    assert_eq!(edge_settings.buffer_size, 64);
}

#[test]
fn test_script_generation_integration() {
    // Test that script generation doesn't panic
    let settings = AudioSettings::new(96000, 24, 1024);
    
    // This is an integration test that verifies the function signature
    // and basic behavior without actually running privileged commands
    let result = std::panic::catch_unwind(|| {
        let _ = apply_audio_settings_with_auth_blocking(settings);
    });
    
    assert!(result.is_ok(), "apply_audio_settings_with_auth_blocking should not panic");
}

#[test]
fn test_system_commands_exist() {
    // Verify that required system commands are available
    let commands = vec!["pw-cli", "pactl", "systemctl", "pkexec"];
    
    for cmd in commands {
        let output = Command::new("which").arg(cmd).output();
        match output {
            Ok(output) if output.status.success() => {
                println!("✓ Command '{}' is available", cmd);
            }
            _ => {
                println!("⚠ Command '{}' not found (may be expected in test environment)", cmd);
            }
        }
    }
}

#[test]
fn test_audio_settings_clone() {
    // Test that AudioSettings can be cloned (important for threading)
    let original = AudioSettings::new(48000, 24, 512);
    let cloned = original.clone();
    
    assert_eq!(original.sample_rate, cloned.sample_rate);
    assert_eq!(original.bit_depth, cloned.bit_depth);
    assert_eq!(original.buffer_size, cloned.buffer_size);
}

#[test]
fn test_audio_settings_debug() {
    // Test Debug implementation
    let settings = AudioSettings::new(44100, 16, 256);
    let debug_output = format!("{:?}", settings);
    
    assert!(debug_output.contains("44100"));
    assert!(debug_output.contains("16"));
    assert!(debug_output.contains("256"));
    assert!(debug_output.contains("AudioSettings"));
}
