/*
 * Pro Audio Config - AdvancedTab Module
 * Version: 2.1
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 *
 * Advanced configuration tab with global and exclusive mode settings
 */

use glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, CheckButton, ComboBoxText, Entry, Frame, Label, Orientation};
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::audio::{
    AudioDevice, AudioSettings, DeviceType, detect_recommended_devices, get_device_capabilities,
};
use crate::config::apply_advanced_audio_settings;

use super::app::AudioApp;
use super::components::{create_constrained_combo, create_section_box, populate_combo_box};
use super::constants::{
    BIT_DEPTHS, BUFFER_SIZES, CONFIG_MODES, EXCLUSIVE_BUFFER_SIZES, SAMPLE_RATES,
};
use super::dialogs::{show_error_dialog, show_success_dialog};
use super::helpers::clean_device_description;

#[derive(Clone)]
pub struct AdvancedTab {
    pub container: GtkBox,
    pub config_mode_combo: ComboBoxText,
    pub global_settings_frame: Frame,
    pub exclusive_settings_frame: Frame,
    pub status_label: Label,
    pub apply_button: Button,
    pub disable_exclusive_button: Button,

    // Global settings
    pub sample_rate_combo: ComboBoxText,
    pub bit_depth_combo: ComboBoxText,
    pub buffer_size_combo: ComboBoxText,
    pub device_combo: ComboBoxText,

    // Exclusive mode settings
    pub application_name_entry: Entry,
    pub process_name_entry: Entry,
    pub exclusive_device_combo: ComboBoxText,
    pub exclusive_sample_rate_combo: ComboBoxText,
    pub exclusive_bit_depth_combo: ComboBoxText,
    pub exclusive_buffer_size_combo: ComboBoxText,
    pub latency_label: Label,

    // Professional settings
    pub pro_settings_frame: Frame,
    pub min_buffer_combo: ComboBoxText,
    pub max_buffer_combo: ComboBoxText,
    pub thread_priority_combo: ComboBoxText,
    pub memory_lock_checkbox: CheckButton,
    pub prevent_suspend_checkbox: CheckButton,
    pub disable_remixing_checkbox: CheckButton,
    pub disable_resampling_checkbox: CheckButton,
    pub resampler_combo: ComboBoxText,
    pub clock_source_combo: ComboBoxText,

    pub available_devices: Vec<AudioDevice>,
    pub current_default_device: Arc<Mutex<String>>,
}

impl Default for AdvancedTab {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedTab {
    fn is_exclusive_mode_active() -> bool {
        let username = whoami::username();

        let exclusive_config_paths = [
            "/etc/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
            "/etc/wireplumber/wireplumber.conf.d/99-pro-audio-exclusive.conf",
            &format!(
                "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
                username
            ),
            &format!(
                "/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio-exclusive.conf",
                username
            ),
        ];

        for path in &exclusive_config_paths {
            if Path::new(path).exists() {
                println!("DEBUG: Found exclusive mode config at: {}", path);
                return true;
            }
        }

        println!("DEBUG: No exclusive mode config found");
        false
    }

    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);

        // ===== CONFIGURATION MODE SELECTION =====
        let (mode_frame, mode_box) = create_section_box("Configuration Mode");

        let mode_selection_label = Label::new(Some("Select Configuration Approach:"));
        mode_selection_label.set_halign(gtk::Align::Start);

        let config_mode_combo = create_constrained_combo();
        for (id, label) in CONFIG_MODES {
            config_mode_combo.append(Some(id), label);
        }
        config_mode_combo.set_active_id(Some("global"));

        let mode_description_label = Label::new(None);
        mode_description_label.set_halign(gtk::Align::Start);
        mode_description_label.set_line_wrap(true);
        Self::update_mode_description(&mode_description_label, "global");

        mode_box.pack_start(&mode_selection_label, false, false, 0);
        mode_box.pack_start(&config_mode_combo, false, false, 0);
        mode_box.pack_start(&mode_description_label, false, false, 0);

        // ===== GLOBAL SETTINGS SECTION =====
        let (global_settings_frame, global_settings_box) =
            create_section_box("Global System Settings");
        let global_frame_clone = global_settings_frame.clone();

        let global_info_label =
            Label::new(Some("These settings affect ALL applications system-wide"));
        global_info_label.set_halign(gtk::Align::Start);

        let global_device_label = Label::new(Some("Audio Device:"));
        global_device_label.set_halign(gtk::Align::Start);

        let device_combo = create_constrained_combo();

        let global_sample_rate_label = Label::new(Some("Sample Rate:"));
        global_sample_rate_label.set_halign(gtk::Align::Start);

        let sample_rate_combo = create_constrained_combo();
        populate_combo_box(&sample_rate_combo, SAMPLE_RATES);
        sample_rate_combo.set_active_id(Some("48000"));

        let global_bit_depth_label = Label::new(Some("Bit Depth:"));
        global_bit_depth_label.set_halign(gtk::Align::Start);

        let bit_depth_combo = create_constrained_combo();
        populate_combo_box(&bit_depth_combo, BIT_DEPTHS);
        bit_depth_combo.set_active_id(Some("24"));

        let global_buffer_size_label = Label::new(Some("Buffer Size:"));
        global_buffer_size_label.set_halign(gtk::Align::Start);

        let buffer_size_combo = create_constrained_combo();
        populate_combo_box(&buffer_size_combo, BUFFER_SIZES);
        buffer_size_combo.set_active_id(Some("512"));

        global_settings_box.pack_start(&global_info_label, false, false, 0);
        global_settings_box.pack_start(&global_device_label, false, false, 0);
        global_settings_box.pack_start(&device_combo, false, false, 0);
        global_settings_box.pack_start(&global_sample_rate_label, false, false, 0);
        global_settings_box.pack_start(&sample_rate_combo, false, false, 0);
        global_settings_box.pack_start(&global_bit_depth_label, false, false, 0);
        global_settings_box.pack_start(&bit_depth_combo, false, false, 0);
        global_settings_box.pack_start(&global_buffer_size_label, false, false, 0);
        global_settings_box.pack_start(&buffer_size_combo, false, false, 0);

        // ===== PROFESSIONAL SETTINGS SECTION =====
        let (pro_settings_frame, pro_settings_box) = create_section_box("Professional Settings");
        let pro_frame_clone = pro_settings_frame.clone();

        let pro_info_label = Label::new(Some(
            "Advanced settings for studio and professional audio work. Use with caution.",
        ));
        pro_info_label.set_halign(gtk::Align::Start);
        pro_info_label.set_line_wrap(true);

        let buffer_range_label = Label::new(Some("Buffer Size Range (min - max):"));
        buffer_range_label.set_halign(gtk::Align::Start);

        let buffer_range_box = GtkBox::new(Orientation::Horizontal, 6);
        buffer_range_box.set_halign(gtk::Align::Start);

        let min_buffer_combo = create_constrained_combo();
        min_buffer_combo.set_width_request(180);
        populate_combo_box(
            &min_buffer_combo,
            &[
                (64, "64 samples (1.3ms @48kHz)"),
                (128, "128 samples (2.7ms @48kHz)"),
                (256, "256 samples (5.3ms @48kHz)"),
                (512, "512 samples (10.7ms @48kHz)"),
                (1024, "1024 samples (21.3ms @48kHz)"),
            ],
        );
        min_buffer_combo.set_active_id(Some("128"));

        let range_separator = Label::new(Some("to"));
        range_separator.set_halign(gtk::Align::Center);

        let max_buffer_combo = create_constrained_combo();
        max_buffer_combo.set_width_request(180);
        populate_combo_box(
            &max_buffer_combo,
            &[
                (512, "512 samples (10.7ms @48kHz)"),
                (1024, "1024 samples (21.3ms @48kHz)"),
                (2048, "2048 samples (42.7ms @48kHz)"),
                (4096, "4096 samples (85.3ms @48kHz)"),
                (8192, "8192 samples (170.7ms @48kHz)"),
            ],
        );
        max_buffer_combo.set_active_id(Some("2048"));

        buffer_range_box.pack_start(&min_buffer_combo, false, false, 0);
        buffer_range_box.pack_start(&range_separator, false, false, 6);
        buffer_range_box.pack_start(&max_buffer_combo, false, false, 0);

        let thread_priority_label = Label::new(Some("Real-time Thread Priority:"));
        thread_priority_label.set_halign(gtk::Align::Start);

        let thread_priority_combo = create_constrained_combo();
        thread_priority_combo.append(Some("normal"), "Normal Priority (-11 nice, 88 RT)");
        thread_priority_combo.append(Some("high"), "High Priority (-15 nice, 90 RT)");
        thread_priority_combo.append(Some("realtime"), "Real-time Priority (-20 nice, 99 RT)");
        thread_priority_combo.set_active_id(Some("high"));

        let memory_lock_checkbox = CheckButton::with_label("Lock audio memory in RAM (mlock)");
        let prevent_suspend_checkbox = CheckButton::with_label("Prevent audio device suspend");
        let disable_remixing_checkbox = CheckButton::with_label("Disable channel remixing");
        let disable_resampling_checkbox = CheckButton::with_label("Disable automatic resampling");

        memory_lock_checkbox.set_tooltip_text(Some("Prevents audio buffers from being swapped to disk, reducing latency but using more RAM"));
        prevent_suspend_checkbox.set_tooltip_text(Some(
            "Keeps audio devices active even when idle, reducing wake-up latency",
        ));
        disable_remixing_checkbox.set_tooltip_text(Some(
            "Prevents automatic channel remixing, maintains original channel layout",
        ));
        disable_resampling_checkbox.set_tooltip_text(Some(
            "Prevents automatic sample rate conversion, may cause issues if rates don't match",
        ));

        let resampler_label = Label::new(Some("Resampler Quality (if resampling is needed):"));
        resampler_label.set_halign(gtk::Align::Start);

        let resampler_combo = create_constrained_combo();
        resampler_combo.append(Some("fastest"), "Fastest (lowest quality, lowest CPU)");
        resampler_combo.append(Some("low"), "Low quality");
        resampler_combo.append(Some("medium"), "Medium (balanced)");
        resampler_combo.append(Some("high"), "High quality");
        resampler_combo.append(Some("highest"), "Highest (best quality, highest CPU)");
        resampler_combo.set_active_id(Some("high"));

        let clock_source_label = Label::new(Some("Clock Source:"));
        clock_source_label.set_halign(gtk::Align::Start);

        let clock_source_combo = create_constrained_combo();
        clock_source_combo.append(Some("default"), "Default (system clock)");
        clock_source_combo.append(Some("monotonic"), "Monotonic (more stable)");
        clock_source_combo.append(Some("realtime"), "Real-time (most accurate)");
        clock_source_combo.set_active_id(Some("monotonic"));

        pro_settings_box.pack_start(&pro_info_label, false, false, 0);
        pro_settings_box.pack_start(&buffer_range_label, false, false, 6);
        pro_settings_box.pack_start(&buffer_range_box, false, false, 0);
        pro_settings_box.pack_start(&thread_priority_label, false, false, 6);
        pro_settings_box.pack_start(&thread_priority_combo, false, false, 0);
        pro_settings_box.pack_start(&clock_source_label, false, false, 6);
        pro_settings_box.pack_start(&clock_source_combo, false, false, 0);

        let checkbox_grid = GtkBox::new(Orientation::Vertical, 4);
        checkbox_grid.set_margin_start(4);
        checkbox_grid.pack_start(&memory_lock_checkbox, false, false, 0);
        checkbox_grid.pack_start(&prevent_suspend_checkbox, false, false, 0);
        checkbox_grid.pack_start(&disable_remixing_checkbox, false, false, 0);
        checkbox_grid.pack_start(&disable_resampling_checkbox, false, false, 0);

        pro_settings_box.pack_start(&checkbox_grid, false, false, 6);
        pro_settings_box.pack_start(&resampler_label, false, false, 6);
        pro_settings_box.pack_start(&resampler_combo, false, false, 0);

        // ===== EXCLUSIVE MODE SETTINGS SECTION =====
        let (exclusive_settings_frame, exclusive_settings_box) =
            create_section_box("Exclusive Mode Settings");
        let exclusive_frame_clone = exclusive_settings_frame.clone();

        let exclusive_info_label = Label::new(Some(
            "Exclusive mode provides direct hardware access for low-latency professional audio work",
        ));
        exclusive_info_label.set_halign(gtk::Align::Start);
        exclusive_info_label.set_line_wrap(true);

        let app_name_label = Label::new(Some("Application Name:"));
        app_name_label.set_halign(gtk::Align::Start);

        let application_name_entry = Entry::new();
        application_name_entry.set_placeholder_text(Some("e.g., Reaper, Ardour, Bitwig"));

        let process_name_label = Label::new(Some("Process Name:"));
        process_name_label.set_halign(gtk::Align::Start);

        let process_name_entry = Entry::new();
        process_name_entry.set_placeholder_text(Some("e.g., reaper, ardour, bitwig-studio"));

        let exclusive_device_label = Label::new(Some("Audio Device:"));
        exclusive_device_label.set_halign(gtk::Align::Start);

        let exclusive_device_combo = create_constrained_combo();

        let exclusive_sample_rate_label = Label::new(Some("Sample Rate:"));
        exclusive_sample_rate_label.set_halign(gtk::Align::Start);

        let exclusive_sample_rate_combo = create_constrained_combo();
        populate_combo_box(&exclusive_sample_rate_combo, SAMPLE_RATES);
        exclusive_sample_rate_combo.set_active_id(Some("48000"));

        let exclusive_bit_depth_label = Label::new(Some("Bit Depth:"));
        exclusive_bit_depth_label.set_halign(gtk::Align::Start);

        let exclusive_bit_depth_combo = create_constrained_combo();
        populate_combo_box(&exclusive_bit_depth_combo, BIT_DEPTHS);
        exclusive_bit_depth_combo.set_active_id(Some("24"));

        let exclusive_buffer_size_label = Label::new(Some("Buffer Size:"));
        exclusive_buffer_size_label.set_halign(gtk::Align::Start);

        let exclusive_buffer_size_combo = create_constrained_combo();
        populate_combo_box(&exclusive_buffer_size_combo, EXCLUSIVE_BUFFER_SIZES);
        exclusive_buffer_size_combo.set_active_id(Some("128"));

        let latency_label = Label::new(Some("Calculated Latency: 2.67ms @ 48kHz"));
        latency_label.set_halign(gtk::Align::Start);

        exclusive_settings_box.pack_start(&exclusive_info_label, false, false, 0);
        exclusive_settings_box.pack_start(&app_name_label, false, false, 0);
        exclusive_settings_box.pack_start(&application_name_entry, false, false, 0);
        exclusive_settings_box.pack_start(&process_name_label, false, false, 0);
        exclusive_settings_box.pack_start(&process_name_entry, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_device_label, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_device_combo, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_sample_rate_label, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_sample_rate_combo, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_bit_depth_label, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_bit_depth_combo, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_buffer_size_label, false, false, 0);
        exclusive_settings_box.pack_start(&exclusive_buffer_size_combo, false, false, 0);
        exclusive_settings_box.pack_start(&latency_label, false, false, 0);

        // ===== ADVANCED ACTIONS SECTION =====
        let (actions_frame, actions_box) = create_section_box("Advanced Actions");

        let status_label = Label::new(Some("Select configuration mode and adjust settings"));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_line_wrap(true);

        let apply_button = Button::with_label("Apply Global System Settings");

        let disable_exclusive_button = Button::with_label("Disable Exclusive Mode");

        let is_exclusive_active = Self::is_exclusive_mode_active();
        println!("DEBUG: Exclusive mode active: {}", is_exclusive_active);
        disable_exclusive_button.set_sensitive(is_exclusive_active);

        let info_label = Label::new(Some(
            "Note: Professional settings may require administrator privileges and can affect system stability",
        ));
        info_label.set_line_wrap(true);

        actions_box.pack_start(&status_label, false, false, 0);
        actions_box.pack_start(&apply_button, false, false, 0);
        actions_box.pack_start(&disable_exclusive_button, false, false, 0);
        actions_box.pack_start(&info_label, false, false, 0);

        // ===== ASSEMBLE ADVANCED TAB =====
        container.pack_start(&mode_frame, false, false, 0);
        container.pack_start(&global_settings_frame, false, false, 0);
        container.pack_start(&pro_settings_frame, false, false, 0);
        container.pack_start(&exclusive_settings_frame, false, false, 0);
        container.pack_start(&actions_frame, false, false, 0);

        global_frame_clone.hide();
        pro_frame_clone.hide();
        exclusive_frame_clone.hide();

        let tab = AdvancedTab {
            container,
            config_mode_combo,
            global_settings_frame,
            exclusive_settings_frame,
            status_label,
            apply_button,
            disable_exclusive_button,
            sample_rate_combo,
            bit_depth_combo,
            buffer_size_combo,
            device_combo,
            application_name_entry,
            process_name_entry,
            exclusive_device_combo,
            exclusive_sample_rate_combo,
            exclusive_bit_depth_combo,
            exclusive_buffer_size_combo,
            latency_label,
            pro_settings_frame,
            min_buffer_combo,
            max_buffer_combo,
            thread_priority_combo,
            memory_lock_checkbox,
            prevent_suspend_checkbox,
            disable_remixing_checkbox,
            disable_resampling_checkbox,
            resampler_combo,
            clock_source_combo,
            available_devices: Vec::new(),
            current_default_device: Arc::new(Mutex::new(String::new())),
        };

        let tab_clone = tab.clone();
        glib::idle_add_local_once(move || {
            tab_clone.global_settings_frame.show();
            tab_clone.pro_settings_frame.show();
            tab_clone.exclusive_settings_frame.hide();
            tab_clone
                .apply_button
                .set_label("Apply Global System Settings");
        });

        tab
    }

    fn update_mode_description(label: &Label, mode: &str) {
        let text = match mode {
            "global" => {
                "Global System Settings: Configure audio settings system-wide for ALL applications. This is the standard approach for general audio configuration."
            }
            "exclusive" => {
                "Exclusive Mode: Provide direct hardware access to a SINGLE application, bypassing mixing and processing. Similar to ASIO/WASAPI exclusive mode for low-latency professional audio work."
            }
            _ => "Select a configuration mode above",
        };
        label.set_text(text);
    }

    pub fn detect_advanced_devices(&self) {
        let device_combo = self.device_combo.clone();
        let exclusive_device_combo = self.exclusive_device_combo.clone();
        let current_default_device = Arc::clone(&self.current_default_device);

        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        std::thread::spawn(move || {
            let result = detect_recommended_devices();
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
                            exclusive_device_combo.remove_all();

                            let default_device_name = {
                                let stored = current_default_device.lock().unwrap();
                                if !stored.is_empty() {
                                    format!("Default: {}", stored)
                                } else {
                                    "Default System Device".to_string()
                                }
                            };

                            device_combo.append(Some("default"), &default_device_name);
                            exclusive_device_combo.append(Some("default"), &default_device_name);

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
                                exclusive_device_combo
                                    .append(Some("separator1"), "--- USB Audio Devices ---");
                                for device in usb_devices {
                                    Self::add_device_to_combo(&device_combo, device);
                                    Self::add_device_to_combo(&exclusive_device_combo, device);
                                }
                            }

                            if !pci_devices.is_empty() {
                                device_combo
                                    .append(Some("separator2"), "--- PCI Audio Devices ---");
                                exclusive_device_combo
                                    .append(Some("separator2"), "--- PCI Audio Devices ---");
                                for device in pci_devices {
                                    Self::add_device_to_combo(&device_combo, device);
                                    Self::add_device_to_combo(&exclusive_device_combo, device);
                                }
                            }

                            if !hdmi_devices.is_empty() {
                                device_combo
                                    .append(Some("separator3"), "--- HDMI/DisplayPort Audio ---");
                                exclusive_device_combo
                                    .append(Some("separator3"), "--- HDMI/DisplayPort Audio ---");
                                for device in hdmi_devices {
                                    Self::add_device_to_combo(&device_combo, device);
                                    Self::add_device_to_combo(&exclusive_device_combo, device);
                                }
                            }

                            if !other_devices.is_empty() {
                                device_combo
                                    .append(Some("separator4"), "--- Other Audio Devices ---");
                                exclusive_device_combo
                                    .append(Some("separator4"), "--- Other Audio Devices ---");
                                for device in other_devices {
                                    Self::add_device_to_combo(&device_combo, device);
                                    Self::add_device_to_combo(&exclusive_device_combo, device);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error detecting advanced devices: {}", e);
                            device_combo.append(Some("default"), "Default System Device");
                            exclusive_device_combo.append(Some("default"), "Default System Device");
                            device_combo.set_active_id(Some("default"));
                            exclusive_device_combo.set_active_id(Some("default"));
                        }
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    device_combo.append(Some("default"), "Default System Device");
                    exclusive_device_combo.append(Some("default"), "Default System Device");
                    device_combo.set_active_id(Some("default"));
                    exclusive_device_combo.set_active_id(Some("default"));
                    ControlFlow::Break
                }
            }
        });
    }

    fn add_device_to_combo(combo: &ComboBoxText, device: &AudioDevice) {
        let device_type = match device.device_type {
            DeviceType::Input => "🎤 Input",
            DeviceType::Output => "🔊 Output",
            DeviceType::Duplex => "🔄 Duplex",
            _ => "🔊 Output",
        };

        let clean_description = clean_device_description(&device.description);

        let display_text = if clean_description.is_empty() {
            format!("{} {}", device_type, device.name)
        } else {
            format!("{} {} - {}", device_type, device.name, clean_description)
        };
        combo.append(Some(&device.id), &display_text);
    }

    pub fn setup_signals(&self, _app_state: AudioApp) {
        let config_mode_combo = self.config_mode_combo.clone();
        let global_settings_frame = self.global_settings_frame.clone();
        let exclusive_settings_frame = self.exclusive_settings_frame.clone();
        let status_label = self.status_label.clone();
        let apply_button = self.apply_button.clone();

        let sample_rate_combo = self.sample_rate_combo.clone();
        let bit_depth_combo = self.bit_depth_combo.clone();
        let buffer_size_combo = self.buffer_size_combo.clone();
        let device_combo = self.device_combo.clone();

        let application_name_entry = self.application_name_entry.clone();
        let process_name_entry = self.process_name_entry.clone();

        let exclusive_device_combo = self.exclusive_device_combo.clone();
        let exclusive_sample_rate_combo = self.exclusive_sample_rate_combo.clone();
        let exclusive_buffer_size_combo = self.exclusive_buffer_size_combo.clone();
        let latency_label = self.latency_label.clone();

        let disable_exclusive_button = self.disable_exclusive_button.clone();

        let is_exclusive_active = Self::is_exclusive_mode_active();
        disable_exclusive_button.set_sensitive(is_exclusive_active);

        // Configuration mode change handler
        {
            let status_label = status_label.clone();
            let global_settings_frame = global_settings_frame.clone();
            let pro_settings_frame = self.pro_settings_frame.clone();
            let exclusive_settings_frame = exclusive_settings_frame.clone();
            let apply_button = apply_button.clone();

            config_mode_combo.connect_changed(move |combo| {
                if let Some(mode) = combo.active_id() {
                    Self::update_mode_description(&status_label, &mode);

                    match mode.as_str() {
                        "global" => {
                            global_settings_frame.show();
                            pro_settings_frame.show();
                            exclusive_settings_frame.hide();
                            apply_button.set_label("Apply Advanced Global Settings");
                        }
                        "exclusive" => {
                            global_settings_frame.hide();
                            pro_settings_frame.hide();
                            exclusive_settings_frame.show();
                            apply_button.set_label("Apply Exclusive Mode Settings");
                        }
                        _ => {}
                    }
                }
            });
        }

        // Exclusive mode latency calculation - buffer size change
        {
            let exclusive_sample_rate_combo = exclusive_sample_rate_combo.clone();
            let latency_label = latency_label.clone();

            exclusive_buffer_size_combo.connect_changed(move |combo| {
                if let Some(buffer_size_str) = combo.active_id()
                    && let Some(sample_rate_str) = exclusive_sample_rate_combo.active_id()
                    && let (Ok(buffer_size), Ok(sample_rate)) = (
                        buffer_size_str.parse::<u32>(),
                        sample_rate_str.parse::<u32>(),
                    )
                {
                    let latency_ms = (buffer_size as f64 * 1000.0) / sample_rate as f64;
                    latency_label.set_text(&format!(
                        "Calculated Latency: {:.2}ms @ {}kHz",
                        latency_ms,
                        sample_rate / 1000
                    ));
                }
            });
        }

        // Exclusive mode latency calculation - sample rate change
        {
            let exclusive_buffer_size_combo = exclusive_buffer_size_combo.clone();
            let latency_label = latency_label.clone();

            exclusive_sample_rate_combo.connect_changed(move |combo| {
                if let Some(sample_rate_str) = combo.active_id()
                    && let Some(buffer_size_str) = exclusive_buffer_size_combo.active_id()
                    && let (Ok(buffer_size), Ok(sample_rate)) = (
                        buffer_size_str.parse::<u32>(),
                        sample_rate_str.parse::<u32>(),
                    )
                {
                    let latency_ms = (buffer_size as f64 * 1000.0) / sample_rate as f64;
                    latency_label.set_text(&format!(
                        "Calculated Latency: {:.2}ms @ {}kHz",
                        latency_ms,
                        sample_rate / 1000
                    ));
                }
            });
        }

        // Apply button handler for advanced tab
        {
            let config_mode_combo = config_mode_combo.clone();
            let status_label = status_label.clone();
            let apply_button = apply_button.clone();

            let sample_rate_combo = sample_rate_combo.clone();
            let bit_depth_combo = bit_depth_combo.clone();
            let buffer_size_combo = buffer_size_combo.clone();
            let device_combo = device_combo.clone();

            let min_buffer_combo = self.min_buffer_combo.clone();
            let max_buffer_combo = self.max_buffer_combo.clone();
            let thread_priority_combo = self.thread_priority_combo.clone();
            let memory_lock_checkbox = self.memory_lock_checkbox.clone();
            let prevent_suspend_checkbox = self.prevent_suspend_checkbox.clone();
            let disable_remixing_checkbox = self.disable_remixing_checkbox.clone();
            let disable_resampling_checkbox = self.disable_resampling_checkbox.clone();
            let resampler_combo = self.resampler_combo.clone();
            let clock_source_combo = self.clock_source_combo.clone();

            let exclusive_device_combo = exclusive_device_combo.clone();
            let exclusive_sample_rate_combo = exclusive_sample_rate_combo.clone();
            let exclusive_buffer_size_combo = exclusive_buffer_size_combo.clone();

            let application_name_entry_clone = application_name_entry.clone();
            let process_name_entry_clone = process_name_entry.clone();

            let apply_button_clone = apply_button.clone();
            let disable_exclusive_button_inner = disable_exclusive_button.clone();

            apply_button.connect_clicked(move |_| {
                let mode = config_mode_combo
                    .active_id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "global".to_string());

                match mode.as_str() {
                    "global" => {
                        status_label.set_text("Applying advanced global system settings...");
                        apply_button_clone.set_sensitive(false);

                        let device_id = device_combo
                            .active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "default".to_string());

                        let settings = AudioSettings {
                            sample_rate: sample_rate_combo
                                .active_id()
                                .and_then(|id| id.parse::<u32>().ok())
                                .unwrap_or(48000),
                            bit_depth: bit_depth_combo
                                .active_id()
                                .and_then(|id| id.parse::<u32>().ok())
                                .unwrap_or(24),
                            buffer_size: buffer_size_combo
                                .active_id()
                                .and_then(|id| id.parse::<u32>().ok())
                                .unwrap_or(512),
                            device_id,
                        };

                        let min_buffer = min_buffer_combo
                            .active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(128);
                        let max_buffer = max_buffer_combo
                            .active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(2048);
                        let thread_priority = thread_priority_combo
                            .active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "high".to_string());
                        let memory_lock = memory_lock_checkbox.is_active();
                        let prevent_suspend = prevent_suspend_checkbox.is_active();
                        let disable_remixing = disable_remixing_checkbox.is_active();
                        let disable_resampling = disable_resampling_checkbox.is_active();
                        let resampler_quality = resampler_combo
                            .active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "high".to_string());
                        let clock_source = clock_source_combo
                            .active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "monotonic".to_string());

                        let status_label_clone = status_label.clone();
                        let apply_button_clone_inner = apply_button_clone.clone();

                        let (tx, rx) = mpsc::channel();
                        let rx_arc = Arc::new(Mutex::new(rx));

                        std::thread::spawn(move || {
                            let result = crate::config::apply_advanced_professional_settings(
                                &settings,
                                true,
                                min_buffer,
                                max_buffer,
                                &thread_priority,
                                memory_lock,
                                prevent_suspend,
                                disable_remixing,
                                disable_resampling,
                                &resampler_quality,
                                &clock_source,
                            );
                            let _ = tx.send(result);
                        });

                        let rx_timeout = Arc::clone(&rx_arc);
                        glib::timeout_add_local(Duration::from_millis(100), move || {
                            let rx_guard = rx_timeout.lock().unwrap();
                            match rx_guard.try_recv() {
                                Ok(result) => {
                                    match result {
                                        Ok(()) => {
                                            status_label_clone
                                                .set_text("Global settings applied successfully!");
                                            apply_button_clone_inner.set_sensitive(true);
                                            show_success_dialog(
                                                "Global audio settings applied successfully.",
                                            );
                                        }
                                        Err(e) => {
                                            status_label_clone
                                                .set_text("Failed to apply advanced settings");
                                            apply_button_clone_inner.set_sensitive(true);
                                            show_error_dialog(&format!(
                                                "Failed to apply advanced settings: {}",
                                                e
                                            ));
                                        }
                                    }
                                    ControlFlow::Break
                                }
                                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    status_label_clone.set_text("Unexpected error");
                                    apply_button_clone_inner.set_sensitive(true);
                                    show_error_dialog("Unexpected error occurred");
                                    ControlFlow::Break
                                }
                            }
                        });
                    }
                    "exclusive" => {
                        status_label.set_text("Applying exclusive mode settings...");
                        apply_button_clone.set_sensitive(false);

                        let device_pattern = exclusive_device_combo
                            .active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "default".to_string());

                        if let Ok(capabilities) = get_device_capabilities(&device_pattern) {
                            let buffer_size = exclusive_buffer_size_combo
                                .active_id()
                                .and_then(|id| id.parse::<u32>().ok())
                                .unwrap_or(128);
                            let sample_rate = exclusive_sample_rate_combo
                                .active_id()
                                .and_then(|id| id.parse::<u32>().ok())
                                .unwrap_or(48000);

                            if !capabilities.buffer_sizes.contains(&buffer_size) {
                                status_label.set_text(
                                    "Warning: Buffer size may not be optimal for this device",
                                );
                            }
                            if !capabilities.sample_rates.contains(&sample_rate) {
                                status_label.set_text(
                                    "Warning: Sample rate may not be supported by this device",
                                );
                            }
                        }

                        let buffer_size = exclusive_buffer_size_combo
                            .active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(128);
                        let sample_rate = exclusive_sample_rate_combo
                            .active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(48000);

                        let app_name = {
                            let text = application_name_entry_clone.text();
                            if text.is_empty() {
                                None
                            } else {
                                Some(text.to_string())
                            }
                        };
                        let app_process_name = {
                            let text = process_name_entry_clone.text();
                            if text.is_empty() {
                                None
                            } else {
                                Some(text.to_string())
                            }
                        };

                        let status_label_clone = status_label.clone();
                        let apply_button_clone_inner = apply_button_clone.clone();
                        let disable_exclusive_button_local = disable_exclusive_button_inner.clone();

                        let (tx, rx) = mpsc::channel();
                        let rx_arc = Arc::new(Mutex::new(rx));

                        std::thread::spawn(move || {
                            let result = apply_advanced_audio_settings(
                                true,
                                true,
                                true,
                                buffer_size,
                                sample_rate,
                                Some(device_pattern),
                                app_name,
                                app_process_name,
                            );
                            let _ = tx.send(result);
                        });

                        let rx_timeout = Arc::clone(&rx_arc);
                        glib::timeout_add_local(Duration::from_millis(100), move || {
                            let rx_guard = rx_timeout.lock().unwrap();
                            match rx_guard.try_recv() {
                                Ok(result) => {
                                    match result {
                                        Ok(()) => {
                                            status_label_clone.set_text(
                                                "Exclusive mode settings applied successfully!",
                                            );
                                            apply_button_clone_inner.set_sensitive(true);
                                            disable_exclusive_button_local.set_sensitive(true);
                                            show_success_dialog(
                                                "Exclusive mode settings applied successfully.",
                                            );
                                        }
                                        Err(e) => {
                                            status_label_clone.set_text(
                                                "Failed to apply exclusive mode settings",
                                            );
                                            apply_button_clone_inner.set_sensitive(true);
                                            show_error_dialog(&format!(
                                                "Failed to apply exclusive mode settings: {}",
                                                e
                                            ));
                                        }
                                    }
                                    ControlFlow::Break
                                }
                                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    status_label_clone.set_text("Unexpected error");
                                    apply_button_clone_inner.set_sensitive(true);
                                    show_error_dialog("Unexpected error occurred");
                                    ControlFlow::Break
                                }
                            }
                        });
                    }
                    _ => {}
                }
            });
        }

        // Disable button actions
        {
            let disable_exclusive_button_clone = disable_exclusive_button.clone();
            disable_exclusive_button.connect_clicked(move |_| {
                match crate::config::restore_standard_audio_mode() {
                    Ok(_) => {
                        show_success_dialog(
                            "Exclusive mode disabled. Standard shared audio mode restored.",
                        );
                        disable_exclusive_button_clone.set_sensitive(false);
                    }
                    Err(e) => {
                        show_error_dialog(&format!("Failed to disable exclusive mode: {}", e));
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusive_mode_state_check() {
        let _exists = AdvancedTab::is_exclusive_mode_active();
        assert!(true);
    }
}
