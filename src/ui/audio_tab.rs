/*
 * Pro Audio Config - AudioTab Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 *
 * Output and Input audio configuration tabs
 */

use glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, ComboBoxText, Label};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::audio::{AudioDevice, AudioSettings, DeviceType, detect_current_audio_settings};
use crate::config::{apply_user_audio_settings, apply_wireplumber_device_config};

use super::app::AudioApp;
use super::components::{create_constrained_combo, create_section_box, populate_combo_box};
use super::constants::{BIT_DEPTHS, BUFFER_SIZES, SAMPLE_RATES};
use super::dialogs::{show_error_dialog, show_success_dialog};
use super::helpers::{clean_device_description, clean_display_text};
use super::preferences::AppPreferences;
use super::tab_types::TabType;

#[derive(Clone)]
pub struct AudioTab {
    pub container: GtkBox,
    pub status_label: Label,
    pub sample_rate_combo: ComboBoxText,
    pub bit_depth_combo: ComboBoxText,
    pub buffer_size_combo: ComboBoxText,
    pub device_combo: ComboBoxText,
    pub current_device_label: Label,
    pub apply_button: Button,
    pub available_devices: Vec<AudioDevice>,
    pub current_default_device: Arc<Mutex<String>>,
    pub tab_type: TabType,
    pub system_wide_checkbox: CheckButton,
    pub preferences: Arc<Mutex<AppPreferences>>,
}

impl AudioTab {
    pub fn new(tab_type: TabType) -> Self {
        let container = GtkBox::new(gtk::Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);

        // ===== DEVICE SECTION =====
        let (device_frame, device_box) = create_section_box(tab_type.device_label());

        let current_device_label = Label::new(Some(&format!(
            "{}: Detecting...",
            tab_type.current_device_prefix()
        )));
        current_device_label.set_halign(gtk::Align::Start);
        current_device_label.set_selectable(true);

        let device_selection_label = Label::new(Some(&format!(
            "Select {} to Configure:",
            tab_type.device_label()
        )));
        device_selection_label.set_halign(gtk::Align::Start);

        let device_combo = create_constrained_combo();

        let selection_info_label = Label::new(Some(&format!(
            "Select an {} device from the dropdown above",
            tab_type.title().to_lowercase()
        )));
        selection_info_label.set_halign(gtk::Align::Start);

        device_box.pack_start(&current_device_label, false, false, 0);
        device_box.pack_start(&device_selection_label, false, false, 0);
        device_box.pack_start(&device_combo, false, false, 0);
        device_box.pack_start(&selection_info_label, false, false, 0);

        // ===== SETTINGS SECTION =====
        let (settings_frame, settings_box) = create_section_box(tab_type.settings_label());
        let preferences = Arc::new(Mutex::new(AppPreferences::load()));

        // Create the system-wide checkbox
        let system_wide_checkbox =
            CheckButton::with_label("Apply system-wide (requires admin password)");

        // DEBUG: Print loaded preferences
        {
            let prefs = preferences.lock().unwrap();
            println!(
                "DEBUG: Loaded preferences - system_wide_config: {}",
                prefs.system_wide_config
            );
        }

        // Set initial state from preferences
        {
            let prefs = preferences.lock().unwrap();
            system_wide_checkbox.set_active(prefs.system_wide_config);
            println!("DEBUG: Setting checkbox to: {}", prefs.system_wide_config);
        }

        // Sample Rate Selection
        let sample_rate_label = Label::new(Some("Sample Rate:"));
        sample_rate_label.set_halign(gtk::Align::Start);

        let sample_rate_combo = create_constrained_combo();
        populate_combo_box(&sample_rate_combo, SAMPLE_RATES);

        if matches!(tab_type, TabType::Output) {
            sample_rate_combo.set_active_id(Some("48000"));
        }

        // Bit Depth Selection
        let bit_depth_label = Label::new(Some("Bit Depth:"));
        bit_depth_label.set_halign(gtk::Align::Start);

        let bit_depth_combo = create_constrained_combo();
        populate_combo_box(&bit_depth_combo, BIT_DEPTHS);

        if matches!(tab_type, TabType::Output) {
            bit_depth_combo.set_active_id(Some("24"));
        }

        // Buffer Size Selection
        let buffer_size_label = Label::new(Some("Buffer Size:"));
        buffer_size_label.set_halign(gtk::Align::Start);

        let buffer_size_combo = create_constrained_combo();
        populate_combo_box(&buffer_size_combo, BUFFER_SIZES);

        if matches!(tab_type, TabType::Output) {
            buffer_size_combo.set_active_id(Some("512"));
        }

        settings_box.pack_start(&sample_rate_label, false, false, 0);
        settings_box.pack_start(&sample_rate_combo, false, false, 0);
        settings_box.pack_start(&bit_depth_label, false, false, 0);
        settings_box.pack_start(&bit_depth_combo, false, false, 0);
        settings_box.pack_start(&buffer_size_label, false, false, 0);
        settings_box.pack_start(&buffer_size_combo, false, false, 0);

        // ===== ACTIONS SECTION =====
        let (actions_frame, actions_box) = create_section_box(tab_type.actions_label());

        let status_label = Label::new(Some(&format!(
            "Ready to configure {} audio settings",
            tab_type.title().to_lowercase()
        )));
        status_label.set_halign(gtk::Align::Start);

        let apply_button = Button::with_label(tab_type.apply_button_label());

        let info_label = Label::new(Some(&format!(
            "Note: Administrator privileges will be requested to apply system {} audio settings",
            tab_type.title().to_lowercase()
        )));
        info_label.set_line_wrap(true);

        actions_box.pack_start(&status_label, false, false, 0);
        actions_box.pack_start(&apply_button, false, false, 0);
        actions_box.pack_start(&info_label, false, false, 0);

        // ===== SYSTEM CONFIG SECTION =====
        let (system_frame, system_box) = create_section_box("Configuration Scope");

        let system_info_label = Label::new(Some(
            "System-wide changes affect all users and require authentication",
        ));
        system_info_label.set_line_wrap(true);

        system_box.pack_start(&system_wide_checkbox, false, false, 0);
        system_box.pack_start(&system_info_label, false, false, 0);

        container.pack_start(&system_frame, false, false, 0);

        // ===== ASSEMBLE TAB =====
        container.pack_start(&device_frame, false, false, 0);
        container.pack_start(&settings_frame, false, false, 0);
        container.pack_start(&actions_frame, false, false, 0);

        Self {
            container,
            status_label,
            sample_rate_combo,
            bit_depth_combo,
            buffer_size_combo,
            device_combo,
            current_device_label,
            apply_button,
            available_devices: Vec::new(),
            current_default_device: Arc::new(Mutex::new(String::new())),
            tab_type,
            system_wide_checkbox,
            preferences,
        }
    }

    /// Detect current default device and store the actual device name
    pub fn detect_current_device(&self) {
        let current_device_label = self.current_device_label.clone();
        let current_default_device = Arc::clone(&self.current_default_device);
        let detect_fn = self.tab_type.detect_current_device_fn();
        let prefix = self.tab_type.current_device_prefix().to_string();

        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        std::thread::spawn(move || {
            let result = detect_fn().and_then(|device_info| {
                Self::extract_actual_device_name(&device_info)
                    .ok_or_else(|| "Could not extract device name".to_string())
            });
            let _ = tx.send(result);
        });

        let rx_timeout = Arc::clone(&rx_arc);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(device_name) => {
                            {
                                let mut stored = current_default_device.lock().unwrap();
                                *stored = device_name.clone();
                            }
                            current_device_label.set_text(&format!("{}: {}", prefix, device_name));
                        }
                        Err(e) => {
                            current_device_label
                                .set_text(&format!("{}: Error detecting - {}", prefix, e));
                        }
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    current_device_label.set_text(&format!("{}: Detection failed", prefix));
                    ControlFlow::Break
                }
            }
        });
    }

    pub fn detect_all_devices(&self) {
        let device_combo = self.device_combo.clone();
        let current_default_device = Arc::clone(&self.current_default_device);
        let detect_fn = self.tab_type.detect_devices_fn();
        let tab_type = self.tab_type.clone();

        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        std::thread::spawn(move || {
            let result = detect_fn();
            let _ = tx.send(result);
        });

        let rx_timeout = Arc::clone(&rx_arc);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(devices) => {
                            device_combo.remove_all();

                            let default_device_name = {
                                let stored = current_default_device.lock().unwrap();
                                if !stored.is_empty() {
                                    format!("Default: {}", stored)
                                } else {
                                    "Default System Device".to_string()
                                }
                            };

                            device_combo.append(Some("default"), &default_device_name);

                            let mut usb_devices = Vec::new();
                            let mut hdmi_devices = Vec::new();
                            let mut pci_devices = Vec::new();
                            let mut other_devices = Vec::new();

                            for device in &devices {
                                let desc_lower = device.description.to_lowercase();
                                let name_lower = device.name.to_lowercase();
                                let id_lower = device.id.to_lowercase();

                                if desc_lower.contains("usb")
                                    || name_lower.contains("usb")
                                    || id_lower.contains("usb")
                                {
                                    usb_devices.push(device);
                                } else if desc_lower.contains("hdmi")
                                    || name_lower.contains("hdmi")
                                    || desc_lower.contains("displayport")
                                    || name_lower.contains("displayport")
                                {
                                    hdmi_devices.push(device);
                                } else if name_lower.contains("pci")
                                    || id_lower.contains("pci")
                                    || desc_lower.contains("pci")
                                {
                                    pci_devices.push(device);
                                } else {
                                    other_devices.push(device);
                                }
                            }

                            if !usb_devices.is_empty() {
                                device_combo
                                    .append(Some("separator1"), "--- USB Audio Devices ---");
                                for device in usb_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            if !pci_devices.is_empty() {
                                device_combo
                                    .append(Some("separator2"), "--- PCI Audio Devices ---");
                                for device in pci_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            if matches!(tab_type, TabType::Output) && !hdmi_devices.is_empty() {
                                device_combo
                                    .append(Some("separator3"), "--- HDMI/DisplayPort Audio ---");
                                for device in hdmi_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            if !other_devices.is_empty() {
                                device_combo
                                    .append(Some("separator4"), "--- Other Audio Devices ---");
                                for device in other_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error detecting {} devices: {}", tab_type.title(), e);
                            device_combo.append(Some("default"), "Default System Device");
                            device_combo.set_active_id(Some("default"));
                        }
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    device_combo.append(Some("default"), "Default System Device");
                    device_combo.set_active_id(Some("default"));
                    ControlFlow::Break
                }
            }
        });
    }

    fn add_device_to_combo(combo: &ComboBoxText, device: &AudioDevice, tab_type: &TabType) {
        let device_type = match device.device_type {
            DeviceType::Input => "🎤 Input",
            DeviceType::Output => "🔊 Output",
            DeviceType::Duplex => "🔄 Duplex",
            _ => match tab_type {
                TabType::Input => "🎤 Input",
                TabType::Output => "🔊 Output",
            },
        };

        let clean_description = clean_device_description(&device.description);

        let display_text = if clean_description.is_empty() {
            format!("{} {}", device_type, device.name)
        } else {
            format!("{} {} - {}", device_type, device.name, clean_description)
        };
        combo.append(Some(&device.id), &display_text);
    }

    pub fn detect_current_settings(&self) {
        let sample_rate_combo = self.sample_rate_combo.clone();
        let bit_depth_combo = self.bit_depth_combo.clone();
        let buffer_size_combo = self.buffer_size_combo.clone();

        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        std::thread::spawn(move || {
            let result = detect_current_audio_settings();
            let _ = tx.send(result);
        });

        let rx_timeout = Arc::clone(&rx_arc);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(settings) => {
                            sample_rate_combo
                                .set_active_id(Some(&settings.sample_rate.to_string()));
                            bit_depth_combo.set_active_id(Some(&settings.bit_depth.to_string()));
                            buffer_size_combo
                                .set_active_id(Some(&settings.buffer_size.to_string()));
                        }
                        Err(e) => {
                            println!("Failed to detect current audio settings: {}", e);
                            sample_rate_combo.set_active_id(Some("48000"));
                            bit_depth_combo.set_active_id(Some("24"));
                            buffer_size_combo.set_active_id(Some("512"));
                        }
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    sample_rate_combo.set_active_id(Some("48000"));
                    bit_depth_combo.set_active_id(Some("24"));
                    buffer_size_combo.set_active_id(Some("512"));
                    ControlFlow::Break
                }
            }
        });
    }

    pub fn setup_signals(&self, app_state: AudioApp) {
        let status_label = self.status_label.clone();
        let apply_button = self.apply_button.clone();
        let sample_rate_combo = self.sample_rate_combo.clone();
        let bit_depth_combo = self.bit_depth_combo.clone();
        let buffer_size_combo = self.buffer_size_combo.clone();
        let device_combo = self.device_combo.clone();
        let current_device_label = self.current_device_label.clone();
        let current_default_device = Arc::clone(&self.current_default_device);
        let apply_fn = self.tab_type.apply_settings_fn();

        // System-wide checkbox handler
        let preferences_clone = Arc::clone(&self.preferences);
        let system_wide_checkbox = self.system_wide_checkbox.clone();

        system_wide_checkbox.connect_toggled(move |checkbox| {
            let system_wide = checkbox.is_active();
            let mut prefs = preferences_clone.lock().unwrap();
            prefs.system_wide_config = system_wide;

            println!(
                "DEBUG: Checkbox toggled - system_wide: {}, saving preferences...",
                system_wide
            );

            if let Err(e) = AppPreferences::save(&prefs) {
                println!("Warning: Failed to save preferences: {}", e);
            } else {
                println!("DEBUG: Preferences saved successfully");
            }

            if system_wide {
                println!("System-wide configuration enabled");
            } else {
                println!("User-specific configuration enabled");
            }
        });

        let tab_type_for_apply = self.tab_type.clone();
        let tab_type_for_device = self.tab_type.clone();
        let preferences_clone = Arc::clone(&self.preferences);

        // Apply button click handler
        self.apply_button.connect_clicked(move |_| {
            let tab_type = tab_type_for_apply.clone();

            let system_wide = {
                let prefs = preferences_clone.lock().unwrap();
                prefs.system_wide_config
            };

            if system_wide {
                status_label.set_text(&format!("Applying system-wide {} settings... (May prompt for admin password)", tab_type.title().to_lowercase()));
            } else {
                status_label.set_text(&format!("Applying user {} settings...", tab_type.title().to_lowercase()));
            }

            apply_button.set_sensitive(false);

            let device_id = device_combo.active_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "default".to_string());

            // Get the actual device name from the system
            let actual_device_name = if device_id == "default" {
                let detect_fn = tab_type.detect_current_device_fn();
                match detect_fn() {
                    Ok(device_info) => {
                        let cleaned = device_info
                            .replace("PipeWire:", "")
                            .replace("PulseAudio:", "")
                            .replace("ALSA:", "")
                            .trim()
                            .to_string();
                        if cleaned.is_empty() || cleaned == "Unknown Output Audio Device"
                            || cleaned == "Unknown Input Audio Device" {
                            None
                        } else {
                            println!("Using detected default device: {}", cleaned);
                            Some(cleaned)
                        }
                    }
                    Err(e) => {
                        println!("Could not detect default device: {}", e);
                        None
                    }
                }
            } else {
                println!("Using selected device: {}", device_id);
                Some(device_id.clone())
            };

            let settings = AudioSettings {
                sample_rate: sample_rate_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(48000),
                bit_depth: bit_depth_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(24),
                buffer_size: buffer_size_combo.active_id()
                    .and_then(|id| id.parse::<u32>().ok())
                    .unwrap_or(512),
                device_id: device_id.clone(),
            };

            let status_label_clone = status_label.clone();
            let apply_button_clone = apply_button.clone();
            let app_state_clone = app_state.clone();

            let (tx, rx) = mpsc::channel();
            let rx_arc = Arc::new(Mutex::new(rx));

            let tab_type_thread = tab_type.clone();
            let actual_device_name_clone = actual_device_name.clone();
            let settings_clone = settings.clone();

            std::thread::spawn(move || {
                let result = if system_wide {
                    apply_fn(settings_clone.clone())
                } else {
                    apply_user_audio_settings(settings_clone.clone(), &tab_type_thread.title().to_lowercase())
                };

                // Also apply device-specific WirePlumber config
                if let Some(ref device_name) = actual_device_name_clone
                    && let Err(e) = apply_wireplumber_device_config(
                        &settings_clone,
                        &tab_type_thread.title().to_lowercase(),
                        Some(device_name)
                    ) {
                        println!("Note: WirePlumber device config failed: {}", e);
                    }

                let _ = tx.send(result);
            });

            let tab_type_timeout = tab_type.clone();
            let status_label_timeout = status_label_clone.clone();
            let apply_button_timeout = apply_button_clone.clone();
            let app_state_timeout = app_state_clone.clone();

            let rx_timeout: Arc<Mutex<mpsc::Receiver<Result<(), String>>>> = Arc::clone(&rx_arc);
            glib::timeout_add_local(Duration::from_millis(100), move || {
                let rx_guard = rx_timeout.lock().unwrap();
                match rx_guard.try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => {
                                status_label_timeout.set_text(&format!("{} settings applied successfully!", tab_type_timeout.title()));
                                apply_button_timeout.set_sensitive(true);
                                show_success_dialog(&format!("{} audio settings applied successfully. The audio system will restart.", tab_type_timeout.title()));

                                let app_state_redetect = app_state_timeout.clone();
                                let status_label_for_closure = status_label_timeout.clone();
                                let tab_type_for_redetect = tab_type_timeout.clone();
                                glib::timeout_add_local(Duration::from_secs(4), move || {
                                    println!("Redetecting audio devices after service restart...");
                                    app_state_redetect.output_tab.detect_all_devices();
                                    app_state_redetect.input_tab.detect_all_devices();
                                    app_state_redetect.output_tab.detect_current_device();
                                    app_state_redetect.input_tab.detect_current_device();
                                    app_state_redetect.output_tab.detect_current_settings();
                                    app_state_redetect.input_tab.detect_current_settings();

                                    status_label_for_closure.set_text(&format!("{} settings applied - devices updated", tab_type_for_redetect.title()));
                                    ControlFlow::Break
                                });
                            }
                            Err(e) => {
                                status_label_timeout.set_text(&format!("Failed to apply {} settings", tab_type_timeout.title().to_lowercase()));
                                apply_button_timeout.set_sensitive(true);
                                show_error_dialog(&format!("Failed to apply {} settings: {}", tab_type_timeout.title().to_lowercase(), e));
                            }
                        }
                        ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        status_label_timeout.set_text("Unexpected error");
                        apply_button_timeout.set_sensitive(true);
                        show_error_dialog("Unexpected error occurred");
                        ControlFlow::Break
                    }
                }
            });
        });

        // Show selection info when device changes
        self.device_combo.connect_changed(move |combo| {
            let tab_type = tab_type_for_device.clone();
            let selection_prefix = tab_type.selection_prefix().to_string();

            if let Some(active_id) = combo.active_id() {
                if active_id.starts_with("separator") || active_id == "no_devices" {
                    combo.set_active_id(Some("default"));
                    return;
                }

                let selection_text = if active_id == "default" {
                    let stored_name = current_default_device.lock().unwrap();
                    if !stored_name.is_empty() {
                        format!(
                            "Selected: Default {} Device ({})",
                            tab_type.title(),
                            stored_name
                        )
                    } else {
                        format!("Selected: Default {} Device", tab_type.title())
                    }
                } else {
                    let display_text = combo.active_text().unwrap_or_default();
                    let clean_text = clean_display_text(&display_text);
                    format!("{}: {}", selection_prefix, clean_text)
                };
                current_device_label.set_text(&selection_text);
            }
        });
    }

    fn extract_actual_device_name(device_info: &str) -> Option<String> {
        let cleaned = device_info
            .replace("PipeWire:", "")
            .replace("PulseAudio:", "")
            .replace("ALSA:", "")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    }
}
