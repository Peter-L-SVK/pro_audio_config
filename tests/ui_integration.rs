//! Integration tests for UI components and data flow

use pro_audio_config::audio::{AudioSettings, AudioDevice, DeviceType};

// Test the device grouping and categorization logic
#[test]
fn test_device_grouping_categorization() {
    // Test the device categorization logic used in the UI for grouping
    let test_devices = vec![
        AudioDevice {
            name: "usb-audio-1".to_string(),
            description: "USB Audio Interface".to_string(),
            id: "alsa:usb1".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "usb-mic".to_string(),
            description: "USB Microphone".to_string(),
            id: "alsa:usb2".to_string(),
            device_type: DeviceType::Input,
            available: true,
        },
        AudioDevice {
            name: "hdmi-output".to_string(),
            description: "HDMI Audio".to_string(),
            id: "alsa:hdmi".to_string(),
            device_type: DeviceType::Output,
            available: true,
        },
        AudioDevice {
            name: "pci-card".to_string(),
            description: "PCI Sound Card".to_string(),
            id: "alsa:pci".to_string(),
            device_type: DeviceType::Duplex,
            available: true,
        },
    ];
    
    // Simulate the grouping logic from the UI
    let mut usb_devices = Vec::new();
    let mut hdmi_devices = Vec::new();
    let mut pci_devices = Vec::new();
    let mut other_devices = Vec::new();
    
    for device in &test_devices {
        let desc_lower = device.description.to_lowercase();
        let name_lower = device.name.to_lowercase();
        let id_lower = device.id.to_lowercase();
        
        if desc_lower.contains("usb") || name_lower.contains("usb") || id_lower.contains("usb") {
            usb_devices.push(device);
        }
        else if desc_lower.contains("hdmi") || name_lower.contains("hdmi") || 
                desc_lower.contains("displayport") || name_lower.contains("displayport") {
            hdmi_devices.push(device);
        }
        else if name_lower.contains("pci") || id_lower.contains("pci") || desc_lower.contains("pci") {
            pci_devices.push(device);
        }
        else {
            other_devices.push(device);
        }
    }
    
    // Verify grouping works correctly
    assert_eq!(usb_devices.len(), 2);
    assert_eq!(hdmi_devices.len(), 1);
    assert_eq!(pci_devices.len(), 1);
    assert_eq!(other_devices.len(), 0);
    
    // Verify device types in groups
    for usb_device in usb_devices {
        assert!(usb_device.description.to_lowercase().contains("usb"));
    }
}

// Test the device display and cleaning functions
#[test]
fn test_device_display_formatting() {
    // Test the device display formatting used in combo boxes
    let device = AudioDevice {
        name: "alsa-device".to_string(),
        description: "USB Audio Device SUSPENDED".to_string(),
        id: "alsa:usb123".to_string(),
        device_type: DeviceType::Output,
        available: true,
    };
    
    // Simulate the display text generation from UI
    let device_type = match device.device_type {
        DeviceType::Input => "🎤 Input",
        DeviceType::Output => "🔊 Output",
        DeviceType::Duplex => "🔄 Duplex", 
        _ => "🔊 Output",
    };
    
    // Clean the description (remove status words like SUSPENDED)
    let clean_description = device.description
        .replace("SUSPENDED", "")
        .replace("RUNNING", "")
        .replace("IDLE", "")
        .trim()
        .trim_end_matches('-')
        .trim()
        .to_string();
    
    let display_text = if clean_description.is_empty() {
        format!("{} {}", device_type, device.name)
    } else {
        format!("{} {} - {}", device_type, device.name, clean_description)
    };
    
    assert_eq!(display_text, "🔊 Output alsa-device - USB Audio Device");
    assert!(!display_text.contains("SUSPENDED"));
}

// Test the tab-specific data structures
#[test]
fn test_tab_data_structures() {
    // Test that we can simulate the tab data structures
    use std::sync::{Arc, Mutex};
    
    // Simulate OutputTab structure
    struct SimulatedOutputTab {
        available_devices: Vec<AudioDevice>,
        current_default_device: Arc<Mutex<String>>,
    }
    
    // Simulate InputTab structure  
    struct SimulatedInputTab {
        available_devices: Vec<AudioDevice>,
        current_default_device: Arc<Mutex<String>>,
    }
    
    let output_tab = SimulatedOutputTab {
        available_devices: vec![
            AudioDevice {
                name: "output-device".to_string(),
                description: "Output Device".to_string(),
                id: "alsa:output".to_string(),
                device_type: DeviceType::Output,
                available: true,
            }
        ],
        current_default_device: Arc::new(Mutex::new("default-output".to_string())),
    };
    
    let input_tab = SimulatedInputTab {
        available_devices: vec![
            AudioDevice {
                name: "input-device".to_string(),
                description: "Input Device".to_string(),
                id: "alsa:input".to_string(),
                device_type: DeviceType::Input,
                available: true,
            }
        ],
        current_default_device: Arc::new(Mutex::new("default-input".to_string())),
    };
    
    // Verify tab-specific data
    assert_eq!(output_tab.available_devices.len(), 1);
    assert_eq!(input_tab.available_devices.len(), 1);
    assert!(output_tab.available_devices[0].device_type == DeviceType::Output);
    assert!(input_tab.available_devices[0].device_type == DeviceType::Input);
    
    // Test shared state access
    let output_device_name = output_tab.current_default_device.lock().unwrap();
    let input_device_name = input_tab.current_default_device.lock().unwrap();
    
    assert_eq!(*output_device_name, "default-output");
    assert_eq!(*input_device_name, "default-input");
}

#[test]
fn test_audio_settings_ui_data_flow() {
    // Test that AudioSettings works well with UI data patterns
    let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
    
    // Simulate UI data passing through channels or cloning
    let settings_for_ui = settings.clone();
    let settings_for_worker = settings.clone();
    
    assert_eq!(settings.sample_rate, settings_for_ui.sample_rate);
    assert_eq!(settings.sample_rate, settings_for_worker.sample_rate);
    
    // Test UI display formatting
    let display_text = format!("{} Hz / {} bit / {} samples", 
        settings.sample_rate, settings.bit_depth, settings.buffer_size);
    assert!(!display_text.is_empty());
    assert!(display_text.contains("48000"));
    assert!(display_text.contains("24"));
    assert!(display_text.contains("512"));
}

#[test]
fn test_dialog_function_safety() {
    // Test that dialog functions can be safely called
    use pro_audio_config::ui::{show_error_dialog, show_success_dialog};
    
    let result = std::panic::catch_unwind(|| {
        // Only test if GTK is initialized (unlikely in tests, but safe to check)
        if gtk::is_initialized() {
            show_error_dialog("Test error message for integration test");
            show_success_dialog("Test success message for integration test");
        } else {
            // This is normal in test environments
            println!("GTK not initialized - skipping actual dialog display");
        }
    });
    
    assert!(result.is_ok(), "Dialog functions should not panic");
}

#[test]
fn test_ui_combo_box_simulation() {
    // Simulate the combo box data structures used in the UI
    let sample_rates = vec![
        (44100, "44.1 kHz - CD Quality"),
        (48000, "48 kHz - Standard Audio"),
        (96000, "96 kHz - High Resolution"),
        (192000, "192 kHz - Studio Quality"),
    ];
    
    let bit_depths = vec![
        (16, "16 bit - CD Quality"),
        (24, "24 bit - High Resolution"),
        (32, "32 bit - Studio Quality"),
    ];
    
    let buffer_sizes = vec![
        (128, "128 samples (2.7ms @48kHz)"),
        (256, "256 samples (5.3ms @48kHz)"),
        (512, "512 samples (10.7ms @48kHz)"),
        (1024, "1024 samples (21.3ms @48kHz)"),
    ];
    
    // Test default selections (like the UI does)
    let default_sample_rate = 48000;
    let default_bit_depth = 24;
    let default_buffer_size = 512;
    
    // Verify defaults exist in our data structures
    assert!(sample_rates.iter().any(|(rate, _)| *rate == default_sample_rate));
    assert!(bit_depths.iter().any(|(depth, _)| *depth == default_bit_depth));
    assert!(buffer_sizes.iter().any(|(size, _)| *size == default_buffer_size));
    
    // Test the parsing logic used in the UI
    let sample_rate = "48000".parse::<u32>().unwrap_or(44100);
    let bit_depth = "24".parse::<u32>().unwrap_or(16);
    let buffer_size = "512".parse::<u32>().unwrap_or(256);
    
    assert_eq!(sample_rate, 48000);
    assert_eq!(bit_depth, 24);
    assert_eq!(buffer_size, 512);
}

#[test]
fn test_ui_fallback_mechanisms() {
    // Test the fallback logic used throughout the UI
    let test_cases = vec![
        // (input, fallback, expected)
        ("48000", 44100, 48000),
        ("invalid", 44100, 44100),
        ("", 44100, 44100),
        ("not_a_number", 48000, 48000),
    ];
    
    for (input, fallback, expected) in test_cases {
        let result = input.parse::<u32>().ok().unwrap_or(fallback);
        assert_eq!(result, expected, "Failed for input '{}' with fallback {}", input, fallback);
    }
}

#[test]
fn test_ui_string_operations() {
    // Test string operations used in UI labels and displays
    
    // Device name cleaning (simulating the private helper function)
    fn clean_device_name(name: &str) -> String {
        name.replace("SUSPENDED", "")
            .replace("RUNNING", "")
            .replace("IDLE", "")
            .trim()
            .trim_end_matches('-')
            .trim()
            .to_string()
    }
    
    let test_cases = vec![
        ("PipeWire Device SUSPENDED", "PipeWire Device"),
        ("ALSA Output - RUNNING", "ALSA Output"),
        ("USB Audio - IDLE", "USB Audio"),
        ("Normal Device", "Normal Device"),
        ("Complex - Device - RUNNING", "Complex - Device"),
    ];
    
    for (input, expected) in test_cases {
        let cleaned = clean_device_name(input);
        assert_eq!(cleaned, expected, "Failed to clean: '{}'", input);
    }
    
    // Test UI label generation
    let settings = AudioSettings::new(96000, 32, 1024, "alsa:usb_device".to_string());
    let status_text = format!("Configured: {}Hz, {}bit, {} samples on {}", 
        settings.sample_rate, settings.bit_depth, settings.buffer_size, settings.device_id);
    
    assert!(status_text.contains("96000"));
    assert!(status_text.contains("32"));
    assert!(status_text.contains("1024"));
    assert!(status_text.contains("usb_device"));
}

#[test]
fn test_ui_thread_safety_patterns() {
    // Test patterns that ensure thread safety in the UI
    use std::sync::{Arc, Mutex};
    
    let settings = Arc::new(Mutex::new(AudioSettings::new(48000, 24, 512, "default".to_string())));
    
    // Simulate UI thread accessing settings
    let ui_settings = Arc::clone(&settings);
    let sample_rate = {
        let guard = ui_settings.lock().unwrap();
        guard.sample_rate
    };
    assert_eq!(sample_rate, 48000);
    
    // Simulate worker thread accessing settings
    let worker_settings = Arc::clone(&settings);
    let locked_settings = worker_settings.lock().unwrap();
    assert_eq!(locked_settings.sample_rate, 48000);
    // Explicitly drop the lock to avoid deadlocks
    drop(locked_settings);
    
    // Test that we can update settings safely
    let update_result = std::panic::catch_unwind(|| {
        let mut settings_guard = settings.lock().unwrap();
        settings_guard.sample_rate = 96000;
    });
    assert!(update_result.is_ok());
    
    let updated_settings = settings.lock().unwrap();
    assert_eq!(updated_settings.sample_rate, 96000);
}

#[test]
fn test_error_handling_patterns() {
    // Test error handling patterns used in the UI
    
    // Simulate the error handling from the apply_audio_settings function
    let error_result: Result<(), String> = Err("Test error message".to_string());
    
    match error_result {
        Ok(()) => {
            // Success case - would update UI to show success
            let status_message = "Settings applied successfully";
            assert!(!status_message.is_empty());
        }
        Err(e) => {
            // Error case - would show error dialog
            assert!(e.contains("Test error message"));
            // Simulate error dialog content extraction
            let error_display = if e.contains("Script failed") {
                e.split("\n\n").nth(1).unwrap_or(&e).to_string()
            } else {
                e
            };
            assert!(!error_display.is_empty());
        }
    }
}
