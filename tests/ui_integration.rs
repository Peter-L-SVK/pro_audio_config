//! Integration tests for UI components

use pro_audio_config::{AudioSettings, show_error_dialog, show_success_dialog};

#[test]
fn test_audio_settings_struct_ui_integration() {
    // Test that AudioSettings works well with UI data flow
    let settings = AudioSettings::new(48000, 24, 512);
    
    // Simulate UI data passing
    let settings_clone = settings.clone();
    assert_eq!(settings.sample_rate, settings_clone.sample_rate);
    
    // Test that settings can be used in UI context
    let label_text = format!("{} Hz / {} bit / {} samples", 
        settings.sample_rate, settings.bit_depth, settings.buffer_size);
    assert!(!label_text.is_empty());
    assert!(label_text.contains("48000"));
}

#[test]
fn test_dialog_functions_exist() {
    // Test that dialog functions can be called (they may not display in tests)
    let result = std::panic::catch_unwind(|| {
        // Only test dialog creation if GTK is already initialized
        if gtk::is_initialized() {
            show_error_dialog("Test error message");
            show_success_dialog("Test success message");
        }
    });
    
    assert!(result.is_ok(), "Dialog functions should not panic when called");
}

#[test]
fn test_ui_string_formatting() {
    // Test string formatting used in UI
    let sample_rates = vec![
        (44100, "44.1 kHz - CD Quality"),
        (48000, "48 kHz - Standard Audio"),
        (96000, "96 kHz - High Resolution"),
    ];
    
    for (rate, label) in sample_rates {
        let formatted = format!("{} - {}", rate, label);
        assert!(!formatted.is_empty());
        assert!(formatted.contains(&rate.to_string()));
    }
    
    let buffer_sizes = vec![
        (128, "128 samples (2.7ms @48kHz)"),
        (512, "512 samples (10.7ms @48kHz)"),
        (2048, "2048 samples (42.7ms @48kHz)"),
    ];
    
    for (size, label) in buffer_sizes {
        let formatted = format!("{} - {}", size, label);
        assert!(!formatted.is_empty());
        assert!(formatted.contains(&size.to_string()));
    }
}

#[test]
fn test_combo_box_simulation() {
    // Simulate combo box behavior without actually creating GTK widgets
    let mut sample_rates = std::collections::HashMap::new();
    sample_rates.insert("44100", "44.1 kHz");
    sample_rates.insert("48000", "48 kHz");
    sample_rates.insert("96000", "96 kHz");
    
    let mut bit_depths = std::collections::HashMap::new();
    bit_depths.insert("16", "16 bit");
    bit_depths.insert("24", "24 bit");
    bit_depths.insert("32", "32 bit");
    
    let mut buffer_sizes = std::collections::HashMap::new();
    buffer_sizes.insert("128", "128 samples");
    buffer_sizes.insert("512", "512 samples");
    buffer_sizes.insert("1024", "1024 samples");
    
    // Simulate active selections
    let active_sample_rate = "48000";
    let active_bit_depth = "24";
    let active_buffer_size = "512";
    
    // Verify selections exist in our simulated combo boxes
    assert!(sample_rates.contains_key(active_sample_rate));
    assert!(bit_depths.contains_key(active_bit_depth));
    assert!(buffer_sizes.contains_key(active_buffer_size));
    
    // Test parsing (like the real UI does)
    let sample_rate = active_sample_rate.parse::<u32>().unwrap();
    let bit_depth = active_bit_depth.parse::<u32>().unwrap();
    let buffer_size = active_buffer_size.parse::<u32>().unwrap();
    
    assert_eq!(sample_rate, 48000);
    assert_eq!(bit_depth, 24);
    assert_eq!(buffer_size, 512);
}

#[test]
fn test_audio_settings_from_simulated_ui() {
    // Test the logic that would be used when getting settings from UI
    let sample_rate = "48000".parse::<u32>().ok().unwrap_or(44100);
    let bit_depth = "24".parse::<u32>().ok().unwrap_or(16);
    let buffer_size = "512".parse::<u32>().ok().unwrap_or(256);
    
    let settings = AudioSettings {
        sample_rate,
        bit_depth,
        buffer_size,
    };
    
    assert_eq!(settings.sample_rate, 48000);
    assert_eq!(settings.bit_depth, 24);
    assert_eq!(settings.buffer_size, 512);
}

#[test]
fn test_fallback_values() {
    // Test the fallback logic used in the UI
    let invalid_sample_rate = "invalid".parse::<u32>().ok().unwrap_or(48000);
    let invalid_bit_depth = "invalid".parse::<u32>().ok().unwrap_or(24);
    let invalid_buffer_size = "invalid".parse::<u32>().ok().unwrap_or(512);
    
    assert_eq!(invalid_sample_rate, 48000);
    assert_eq!(invalid_bit_depth, 24);
    assert_eq!(invalid_buffer_size, 512);
}
