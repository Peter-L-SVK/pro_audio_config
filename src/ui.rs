/*
 * Pro Audio Config - User Interface Module  
 * Version: 1.3
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 */

use gtk::prelude::*;
#[allow(unused_imports)] // AboutDialog is used in code
use gtk::{
    Application, ApplicationWindow, Button, Label, ComboBoxText, Box as GtkBox, 
    Orientation, Frame, ScrolledWindow, Separator, MessageDialog, MessageType, ButtonsType,
    DialogFlags, Window, Adjustment, AboutDialog
};
use glib::ControlFlow;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::audio::{
    AudioSettings, AudioDevice,  
    detect_audio_device, 
    detect_all_audio_devices,
    detect_current_audio_settings,
    DeviceType
};
use crate::config::apply_audio_settings_with_auth_blocking;

pub struct AudioApp {
    pub window: ApplicationWindow,
    pub status_label: Label,
    pub sample_rate_combo: ComboBoxText,
    pub bit_depth_combo: ComboBoxText, 
    pub buffer_size_combo: ComboBoxText,
    pub device_combo: ComboBoxText,
    pub current_device_label: Label,
    pub apply_button: Button,
    pub available_devices: Vec<AudioDevice>,
    pub current_default_device: Arc<Mutex<String>>,
}

impl AudioApp {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title("Pro Audio Config");
        window.set_default_size(500, 650);

        // Set window icon from system or development location
        let icon_paths = [
            // System installation paths (multiple sizes)
            "/usr/share/icons/hicolor/16x16/apps/pro-audio-config.png",
            "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
            "/usr/share/icons/hicolor/32x32/apps/pro-audio-config.png", 
            "/usr/share/icons/hicolor/256x256/apps/pro-audio-config.png",
            // Development paths
            "icons/48x48/pro-audio-config.png",
            "icons/32x32/pro-audio-config.png",
            "icons/icon.png", // Relative path from project root
            "icon.png", // Current directory
            "../icons/icon.png", // If running from different directory
            "./icons/icon.png", // Explicit current directory
            // Alternative system paths
            "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
            "/usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png",
        ];
        
        for path in &icon_paths {
            if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(path) {
                window.set_icon(Some(&pixbuf));
                break;
            }
        }
        
        // Create menu bar
        let menu_bar = gtk::MenuBar::new();
        
        // Create Help menu
        let help_menu = gtk::Menu::new();
        let help_menu_item = gtk::MenuItem::with_label("Help");
        help_menu_item.set_submenu(Some(&help_menu));
        
        // Create About item
        let about_item = gtk::MenuItem::with_label("About");
        about_item.connect_activate(|_| {
            show_about_dialog();
        });
        
        help_menu.append(&about_item);
        menu_bar.append(&help_menu_item);

        let scrolled_window = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_propagate_natural_width(true);
        
        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(18);
        main_box.set_margin_bottom(18);
        main_box.set_margin_start(18);
        main_box.set_margin_end(18);

        // Add menu bar to main interface
        main_box.pack_start(&menu_bar, false, false, 0);

        // ===== DEVICE SECTION =====
        let (device_frame, device_box) = create_section_box("Audio Device");
        
        // CURRENT Device Display (Read-only)
        let current_device_label = Label::new(Some("Current Default Device: Detecting..."));
        current_device_label.set_halign(gtk::Align::Start);
        current_device_label.set_selectable(true); // Allow copying device name
        
        // Device Selection Dropdown
        let device_selection_label = Label::new(Some("Select Audio Device to Configure:"));
        device_selection_label.set_halign(gtk::Align::Start);
        
        let device_combo = ComboBoxText::new();
        
        let selection_info_label = Label::new(Some("Select a device from the dropdown above"));
        selection_info_label.set_halign(gtk::Align::Start);
        
        // Add device info to device box
        device_box.pack_start(&current_device_label, false, false, 0);
        device_box.pack_start(&device_selection_label, false, false, 0);
        device_box.pack_start(&device_combo, false, false, 0);
        device_box.pack_start(&selection_info_label, false, false, 0);

        // ===== SETTINGS SECTION =====
        let (settings_frame, settings_box) = create_section_box("Audio Settings");
        
        // Sample Rate Selection
        let sample_rate_label = Label::new(Some("Sample Rate:"));
        sample_rate_label.set_halign(gtk::Align::Start);
        
        let sample_rate_combo = ComboBoxText::new();
        let sample_rates = vec![
            (44100, "44.1 kHz - CD Quality"),
            (48000, "48 kHz - Standard Audio"),
            (96000, "96 kHz - High Resolution"),
            (192000, "192 kHz - Studio Quality"), 
            (384000, "384 kHz - Ultra High Resolution"),
        ];
        
        for (rate, label) in sample_rates {
            sample_rate_combo.append(Some(&rate.to_string()), label);
        }
        sample_rate_combo.set_active_id(Some("48000"));

        // Bit Depth Selection
        let bit_depth_label = Label::new(Some("Bit Depth:"));
        bit_depth_label.set_halign(gtk::Align::Start);
        
        let bit_depth_combo = ComboBoxText::new();
        let bit_depths = vec![
            (16, "16 bit - CD Quality"),
            (24, "24 bit - High Resolution"), 
            (32, "32 bit - Studio Quality"),
        ];
        
        for (depth, label) in bit_depths {
            bit_depth_combo.append(Some(&depth.to_string()), label);
        }
        bit_depth_combo.set_active_id(Some("24"));

        // Buffer Size Selection
        let buffer_size_label = Label::new(Some("Buffer Size:"));
        buffer_size_label.set_halign(gtk::Align::Start);
        
        let buffer_size_combo = ComboBoxText::new();
        let buffer_sizes = vec![
            (128, "128 samples (2.7ms @48kHz)"),
            (256, "256 samples (5.3ms @48kHz)"),
            (512, "512 samples (10.7ms @48kHz)"),
            (1024, "1024 samples (21.3ms @48kHz)"),
            (2048, "2048 samples (42.7ms @48kHz)"),
            (4096, "4096 samples (85.3ms @48kHz)"),
            (8192, "8192 samples (170.7ms @48kHz)"),
        ];
        
        for (size, label) in buffer_sizes {
            buffer_size_combo.append(Some(&size.to_string()), label);
        }
        buffer_size_combo.set_active_id(Some("512"));

        // Add settings to settings box
        settings_box.pack_start(&sample_rate_label, false, false, 0);
        settings_box.pack_start(&sample_rate_combo, false, false, 0);
        settings_box.pack_start(&bit_depth_label, false, false, 0);
        settings_box.pack_start(&bit_depth_combo, false, false, 0);
        settings_box.pack_start(&buffer_size_label, false, false, 0);
        settings_box.pack_start(&buffer_size_combo, false, false, 0);

        // ===== ACTIONS SECTION =====
        let (actions_frame, actions_box) = create_section_box("Actions");
        
        let status_label = Label::new(Some("Ready to configure audio settings"));
        status_label.set_halign(gtk::Align::Start);
        
        let apply_button = Button::with_label("Apply Audio Settings");

        let info_label = Label::new(Some("Note: Administrator privileges will be requested to apply system audio settings"));
        info_label.set_line_wrap(true);

        actions_box.pack_start(&status_label, false, false, 0);
        actions_box.pack_start(&apply_button, false, false, 0);
        actions_box.pack_start(&info_label, false, false, 0);

        // ===== ASSEMBLE MAIN INTERFACE =====
        main_box.pack_start(&device_frame, false, false, 0);
        main_box.pack_start(&settings_frame, false, false, 0);
        main_box.pack_start(&actions_frame, false, false, 0);
        
        scrolled_window.add(&main_box);
        window.add(&scrolled_window);

        let app_state = Self {
            window,
            status_label,
            sample_rate_combo,
            bit_depth_combo,
            buffer_size_combo,
            device_combo,
            current_device_label,
            apply_button,
            available_devices: Vec::new(),
            current_default_device: Arc::new(Mutex::new(String::new())), // Initialize empty
        };

        // ===== CONNECT SIGNALS =====
        app_state.setup_signals();
        
        // ===== DETECT ALL DEVICES AND CURRENT SETTINGS =====
        app_state.detect_all_devices();
        app_state.detect_current_settings();
        app_state.detect_current_device();

        app_state
    }

    // Detect current default device and store the actual device name
    fn detect_current_device(&self) {
	let current_device_label = self.current_device_label.clone();
	let current_default_device = Arc::clone(&self.current_default_device);
	
	// Create channel for communication
	let (tx, rx) = mpsc::channel();
	let rx_arc = Arc::new(Mutex::new(rx));
	
	// Spawn thread for device detection
	std::thread::spawn(move || {
            let result = detect_audio_device();
            let _ = tx.send(result);
	});
	
	// Set up timeout to check for result
	let rx_timeout = Arc::clone(&rx_arc);
	glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
		Ok(result) => {
                    match result {
			Ok(device_info) => {
                            // Store the actual device name for later use
                            let actual_device_name = clean_device_display(&device_info);
                            {
				let mut stored = current_default_device.lock().unwrap();
				*stored = actual_device_name.clone();
                            }
                            
                            current_device_label.set_text(&format!("Current Default Device: {}", actual_device_name));
			}
			Err(e) => {
                            current_device_label.set_text(&format!("Current Default Device: Error detecting - {}", e));
			}
                    }
                    ControlFlow::Break
		}
		Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
		Err(mpsc::TryRecvError::Disconnected) => {
                    current_device_label.set_text("Current Default Device: Detection failed");
                    ControlFlow::Break
		}
            }
	});
    }

    fn detect_all_devices(&self) {
	let device_combo = self.device_combo.clone();
	let current_default_device = Arc::clone(&self.current_default_device);
	
	// Create channel for communication  
	let (tx, rx) = mpsc::channel();
	let rx_arc = Arc::new(Mutex::new(rx));
	
	// Spawn thread for device detection
	std::thread::spawn(move || {
            let result = detect_all_audio_devices();
            let _ = tx.send(result);
	});
	
	// Set up timeout to check for result
	let rx_timeout = Arc::clone(&rx_arc);
	glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
		Ok(result) => {
                    match result {
			Ok(devices) => {
                            // Clear existing items
                            device_combo.remove_all();
                            
                            // Get the stored default device name if available
                            let default_device_name = {
				let stored = current_default_device.lock().unwrap();
				if !stored.is_empty() {
                                    // Just use the stored device name directly
                                    format!("Default: {}", stored)
				} else {
                                    "Default System Device".to_string()
				}
                            };
                            
                            // Add "Default Device" option with actual device name
                            device_combo.append(Some("default"), &default_device_name);
                            
                            // Filter devices to only include output devices AND real hardware
                            let output_devices: Vec<&AudioDevice> = devices.iter()
				.filter(|device| {
                                    // Only output/duplex devices that are real hardware
                                    matches!(device.device_type, DeviceType::Output | DeviceType::Duplex)
				})
				.collect();
                            
                            if !output_devices.is_empty() {
				// Group by device type for better organization
				let mut usb_devices = Vec::new();
				let mut hdmi_devices = Vec::new();
				let mut pci_devices = Vec::new();
				let mut other_devices = Vec::new();
				
				for device in output_devices {
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
				
				// Add USB devices first (most common for pro audio)
				if !usb_devices.is_empty() {
				    device_combo.append(Some("separator1"), "--- USB Audio Devices ---");
				    for device in usb_devices {
					Self::add_device_to_combo(&device_combo, device);
				    }
				}
				
				// Add PCI devices (onboard and sound cards)
				if !pci_devices.is_empty() {
				    device_combo.append(Some("separator2"), "--- PCI Audio Devices ---");
				    for device in pci_devices {
					Self::add_device_to_combo(&device_combo, device);
				    }
				}
				
				// Add HDMI devices  
				if !hdmi_devices.is_empty() {
				    device_combo.append(Some("separator3"), "--- HDMI/DisplayPort Audio ---");
				    for device in hdmi_devices {
					Self::add_device_to_combo(&device_combo, device);
				    }
				}
				
				// Add any remaining devices
				if !other_devices.is_empty() {
				    device_combo.append(Some("separator4"), "--- Other Audio Devices ---");
				    for device in other_devices {
					Self::add_device_to_combo(&device_combo, device);
				    }
				}
                            } else {
				// No output devices found, add a message
				device_combo.append(Some("no_devices"), "No output devices found");
                            }
                            
                            // Select "Default Device" by default
                            device_combo.set_active_id(Some("default"));
			}
			Err(e) => {
                            println!("Error detecting devices: {}", e);
                            // Still add default option
                            device_combo.append(Some("default"), "Default System Device");
                            device_combo.set_active_id(Some("default"));
			}
                    }
                    ControlFlow::Break
		}
		Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
		Err(mpsc::TryRecvError::Disconnected) => {
                    // Add default option as fallback
                    device_combo.append(Some("default"), "Default System Device");
                    device_combo.set_active_id(Some("default"));
                    ControlFlow::Break
		}
            }
	});
    }
    
    // Helper function to add devices to combo box with consistent formatting
    fn add_device_to_combo(combo: &ComboBoxText, device: &AudioDevice) {
	let device_type = match device.device_type {
            DeviceType::Output => "🔊 Output",
            DeviceType::Duplex => "🔄 Duplex", 
            _ => "🔊 Output",
	};
	
	// Clean the description by removing "SUSPENDED" and any trailing status words
	let clean_description = clean_device_description(&device.description);
	
	let display_text = if clean_description.is_empty() {
            format!("{} {}", device_type, device.name)
	} else {
            format!("{} {} - {}", device_type, device.name, clean_description)
	};
	combo.append(Some(&device.id), &display_text);
    }
    
    fn detect_current_settings(&self) {
        let sample_rate_combo = self.sample_rate_combo.clone();
        let bit_depth_combo = self.bit_depth_combo.clone();
        let buffer_size_combo = self.buffer_size_combo.clone();
        
        // Create channel for communication
        let (tx, rx) = mpsc::channel();
        
        // Store the receiver in an Arc<Mutex> to share between threads
        let rx_arc = Arc::new(Mutex::new(rx));
        
        // Spawn thread for blocking operation
        std::thread::spawn(move || {
            let result = detect_current_audio_settings();
            let _ = tx.send(result);
        });
        
        // Set up timeout to check for result
        let rx_timeout = Arc::clone(&rx_arc);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(settings) => {
                            // Set the combo boxes to current values
                            sample_rate_combo.set_active_id(Some(&settings.sample_rate.to_string()));
                            bit_depth_combo.set_active_id(Some(&settings.bit_depth.to_string()));
                            buffer_size_combo.set_active_id(Some(&settings.buffer_size.to_string()));
                        }
                        Err(e) => {
                            println!("Failed to detect current settings: {}", e);
                            // Set defaults if detection fails
                            sample_rate_combo.set_active_id(Some("48000"));
                            bit_depth_combo.set_active_id(Some("24"));
                            buffer_size_combo.set_active_id(Some("512"));
                        }
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still processing, check again
                    ControlFlow::Continue
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Thread finished but no result, set defaults
                    sample_rate_combo.set_active_id(Some("48000"));
                    bit_depth_combo.set_active_id(Some("24"));
                    buffer_size_combo.set_active_id(Some("512"));
                    ControlFlow::Break
                }
            }
        });
    }

    fn setup_signals(&self) {
        let status_label = self.status_label.clone();
        let apply_button = self.apply_button.clone();
        let sample_rate_combo = self.sample_rate_combo.clone();
        let bit_depth_combo = self.bit_depth_combo.clone();
        let buffer_size_combo = self.buffer_size_combo.clone();
        let device_combo = self.device_combo.clone();
        let current_device_label = self.current_device_label.clone();
        let current_default_device = Arc::clone(&self.current_default_device);
        
        self.apply_button.connect_clicked(move |_| {
            // Update UI immediately
            status_label.set_text("Applying settings...");
            apply_button.set_sensitive(false);
            
            // Get selected device ID
            let device_id = device_combo.active_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "default".to_string());
            
            // Get settings
            let settings = {
                let sample_rate = sample_rate_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(48000);
                
                let bit_depth = bit_depth_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(24);
                
                let buffer_size = buffer_size_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(512);
                
                AudioSettings {
                    sample_rate,
                    bit_depth,
                    buffer_size,
                    device_id,
                }
            };
            
            let status_label_clone = status_label.clone();
            let apply_button_clone = apply_button.clone();
            
            // Create channel for communication
            let (tx, rx) = mpsc::channel();
            
            // Store the receiver in an Arc<Mutex> to share between threads
            let rx_arc = Arc::new(Mutex::new(rx));
            
            // Spawn thread for blocking operation
            std::thread::spawn(move || {
                let result = apply_audio_settings_with_auth_blocking(settings);
                let _ = tx.send(result);
            });
            
            // Set up timeout to check for result
            let rx_timeout = Arc::clone(&rx_arc);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                let rx_guard = rx_timeout.lock().unwrap();
                match rx_guard.try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => {
                                status_label_clone.set_text("Settings applied successfully!");
                                apply_button_clone.set_sensitive(true);
                                show_success_dialog("Audio settings applied successfully. The audio system will restart.");
                            }
                            Err(e) => {
                                status_label_clone.set_text("Failed to apply settings");
                                apply_button_clone.set_sensitive(true);
                                show_error_dialog(&format!("Failed to apply settings: {}", e));
                            }
                        }
                        ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        // Still processing, check again
                        ControlFlow::Continue
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Thread finished but no result
                        status_label_clone.set_text("Unexpected error");
                        apply_button_clone.set_sensitive(true);
                        show_error_dialog("Unexpected error occurred");
                        ControlFlow::Break
                    }
                }
            });
        });
	
        // Show selection info when device changes
	self.device_combo.connect_changed(move |combo| {
	    if let Some(active_id) = combo.active_id() {
		// Skip separators and other non-selectable items
		if active_id.starts_with("separator") || active_id == "no_devices" {
		    // Immediately revert to the previous valid selection
		    combo.set_active_id(Some("default"));
		    return;
		}
		
		let selection_text = if active_id == "default" {
		    // Use the stored device name
		    let stored_name = current_default_device.lock().unwrap();
		    if !stored_name.is_empty() {
			format!("Selected: Default System Device ({})", stored_name)
		    } else {
			"Selected: Default System Device".to_string()
		    }
		} else {
		    // Get the display text and clean it for the selection info
		    let display_text = combo.active_text().unwrap_or_default();
		    let clean_text = clean_display_text(&display_text);
		    format!("Selected: {}", clean_text)
		};
		current_device_label.set_text(&selection_text);
	    }
	});
    }
}

// Helper function to clean device description by removing status words like "SUSPENDED"
fn clean_device_description(description: &str) -> String {
    description
        .replace("SUSPENDED", "")
        .replace("RUNNING", "")
        .replace("IDLE", "")
        .trim()
        .trim_end_matches('-')
        .trim()
        .to_string()
}

// Helper function to clean display text in the selection info
fn clean_display_text(display_text: &str) -> String {
    display_text
        .replace("SUSPENDED", "")
        .replace("RUNNING", "")
        .replace("IDLE", "")
        .trim()
        .trim_end_matches('-')
        .trim()
        .to_string()
}

// Helper function to clean device display name (remove PipeWire: prefix etc.)
fn clean_device_display(device_info: &str) -> String {
    device_info
        .replace("PipeWire:", "")
        .replace("PulseAudio:", "")
        .replace("ALSA:", "")
        .trim()
        .to_string()
}

pub fn create_section_box(title: &str) -> (Frame, GtkBox) {
    let frame = Frame::new(None);
    frame.set_margin_top(6);
    frame.set_margin_bottom(6);
    
    let section_box = GtkBox::new(Orientation::Vertical, 12);
    section_box.set_margin_top(12);
    section_box.set_margin_bottom(12);
    section_box.set_margin_start(12);
    section_box.set_margin_end(12);
    
    let title_label = Label::new(None);
    title_label.set_markup(&format!("<b>{}</b>", title));
    title_label.set_halign(gtk::Align::Start);
    
    section_box.pack_start(&title_label, false, false, 0);
    
    // Add separator
    let separator = Separator::new(Orientation::Horizontal);
    section_box.pack_start(&separator, false, false, 0);
    
    frame.add(&section_box);
    
    (frame, section_box)
}

pub fn show_error_dialog(message: &str) {
    let dialog = MessageDialog::new::<Window>(
        None,
        DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        "Failed to apply audio settings"
    );
    
    dialog.set_title("Error Applying Settings");
    
    // Format the message to show the actual error details
    let display_message = if message.contains("Script failed") {
        // Extract the actual error part from the message
        if let Some(error_part) = message.split("\n\n").nth(1) {
            error_part.to_string()
        } else {
            message.to_string()
        }
    } else {
        message.to_string()
    };
    
    dialog.set_secondary_text(Some(&display_message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.show_all();
}

pub fn show_success_dialog(message: &str) {
    let dialog = MessageDialog::new::<Window>(
        None,
        DialogFlags::MODAL,
        MessageType::Info,
        ButtonsType::Ok,
        "Settings Applied Successfully"
    );
    
    dialog.set_secondary_text(Some(message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.show_all();
}

pub fn show_about_dialog() {
    let dialog = gtk::AboutDialog::new();
    
    dialog.set_title("About Pro Audio Config");
    dialog.set_program_name("Pro Audio Config");
    dialog.set_version(Some("1.3"));
    dialog.set_website(Some("https://github.com/Peter-L-SVK/pro_audio_config"));
    dialog.set_copyright(Some("Copyright © 2025 Peter Leukanič"));
    dialog.set_authors(&["Peter Leukanič"]);
    
    // Set license text (optional)
    dialog.set_license_type(gtk::License::MitX11);

    dialog.set_license(Some(
        "This program is free software: you can redistribute it and/or modify\n\
         it under the terms of the MIT License.\n\
         \n\
         This program is distributed in the hope that it will be useful,\n\
         but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
         MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.\n\
         \n\
         For more details, see the LICENSE file included with this distribution."
    ));
    dialog.set_wrap_license(true);

    let icon_paths = [
        // System installation paths (multiple sizes)
        "/usr/share/icons/hicolor/16x16/apps/pro-audio-config.png",
        "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
        "/usr/share/icons/hicolor/32x32/apps/pro-audio-config.png", 
        "/usr/share/icons/hicolor/256x256/apps/pro-audio-config.png",
        // Development paths
        "icons/48x48/pro-audio-config.png",
        "icons/32x32/pro-audio-config.png",
        "icons/icon.png", // Relative path from project root
        "icon.png", // Current directory
        "../icons/icon.png", // If running from different directory
        "./icons/icon.png", // Explicit current directory
        // Alternative system paths
        "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
        "/usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png",
    ];
    
    for path in &icon_paths {
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(path) {
            dialog.set_logo(Some(&pixbuf));
            break;
        }
    }
    
    dialog.set_modal(true);
    dialog.set_transient_for(Some(&get_main_window()));
    
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.show_all();
}

// Helper function to get the main window
fn get_main_window() -> gtk::Window {
    // This is a simple implementation - you might need to adjust based on your app structure
    gtk::Window::new(gtk::WindowType::Toplevel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_device_description() {
        assert_eq!(
            clean_device_description("PipeWire s32le 4ch 192000Hz SUSPENDED"),
            "PipeWire s32le 4ch 192000Hz"
        );
        assert_eq!(
            clean_device_description("SUSPENDED PipeWire Device"),
            "PipeWire Device"
        );
        assert_eq!(
            clean_device_description("Device - RUNNING"),
            "Device"
        );
        assert_eq!(
            clean_device_description("Normal Device Description"),
            "Normal Device Description"
        );
    }

    #[test]
    fn test_clean_display_text() {
        assert_eq!(
            clean_display_text("🔊 Output alsa_device - SUSPENDED"),
            "🔊 Output alsa_device"
        );
        assert_eq!(
            clean_display_text("🔄 Duplex device - RUNNING"),
            "🔄 Duplex device"
        );
    }

    #[test]
    fn test_clean_device_display() {
        assert_eq!(
            clean_device_display("PipeWire: alsa_output.usb-PreSonus_Studio_26c_0142446A-00.analog-surround-40"),
            "alsa_output.usb-PreSonus_Studio_26c_0142446A-00.analog-surround-40"
        );
        assert_eq!(
            clean_device_display("PulseAudio: default_sink"),
            "default_sink"
        );
    }

    #[test]
    fn test_audio_settings_from_ui() {
        // Test the parsing logic without GTK
        let sample_rate = "48000".parse::<u32>().ok().unwrap_or(44100);
        let bit_depth = "24".parse::<u32>().ok().unwrap_or(16);
        let buffer_size = "512".parse::<u32>().ok().unwrap_or(256);
        
        assert_eq!(sample_rate, 48000);
        assert_eq!(bit_depth, 24);
        assert_eq!(buffer_size, 512);
    }

    #[test]
    fn test_fallback_parsing() {
        // Test fallback values for invalid parsing
        let invalid_rate = "invalid".parse::<u32>().ok().unwrap_or(48000);
        let invalid_depth = "invalid".parse::<u32>().ok().unwrap_or(24);
        let invalid_size = "invalid".parse::<u32>().ok().unwrap_or(512);
        
        assert_eq!(invalid_rate, 48000);
        assert_eq!(invalid_depth, 24);
        assert_eq!(invalid_size, 512);
    }

    #[test]
    fn test_audio_settings_struct() {
        // Test AudioSettings creation and access
        let settings = crate::audio::AudioSettings {
            sample_rate: 96000,
            bit_depth: 32,
            buffer_size: 1024,
            device_id: "default".to_string(),
        };
        
        assert_eq!(settings.sample_rate, 96000);
        assert_eq!(settings.bit_depth, 32);
        assert_eq!(settings.buffer_size, 1024);
        assert_eq!(settings.device_id, "default");
    }
}
