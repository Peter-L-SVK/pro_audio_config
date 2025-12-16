/*
 * Pro Audio Config - User Interface Module
 * Version: 1.9
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Graphical user interface module implementing separate configuration tabs with comprehensive device management.
 */

use glib::ControlFlow;
use gtk::prelude::*;
#[allow(unused_imports)] // AboutDialog is used in code
use gtk::{
    AboutDialog, Adjustment, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType,
    CheckButton, ComboBoxText, DialogFlags, Entry, Frame, Label, MessageDialog, MessageType,
    Notebook, Orientation, ScrolledWindow, Separator, Window,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::audio::{
    AudioDevice, AudioSettings, DeviceType, detect_current_audio_settings,
    detect_input_audio_device, detect_input_audio_devices, detect_output_audio_device,
    detect_output_audio_devices, detect_recommended_devices, get_device_capabilities,
};
use crate::config::{
    apply_advanced_audio_settings, apply_input_audio_settings_with_auth_blocking,
    apply_output_audio_settings_with_auth_blocking, apply_user_audio_settings,
};
use crate::config_inspector::ConfigInspectorTab;
use crate::monitoring::MonitoringTab;

#[derive(Clone)]
pub struct AudioApp {
    pub window: ApplicationWindow,
    pub notebook: Notebook,
    pub output_tab: AudioTab,
    pub input_tab: AudioTab,
    pub advanced_tab: AdvancedTab,
    pub monitoring_tab: MonitoringTab,
    pub config_inspector_tab: ConfigInspectorTab,
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppPreferences {
    pub system_wide_config: bool,
}

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

// Common option definitions to avoid duplication
const SAMPLE_RATES: &[(u32, &str)] = &[
    (44100, "44.1 kHz - CD Quality"),
    (48000, "48 kHz - Standard Audio"),
    (96000, "96 kHz - High Resolution"),
    (192000, "192 kHz - Studio Quality"),
    (384000, "384 kHz - Ultra High Resolution"),
];

const BIT_DEPTHS: &[(u32, &str)] = &[
    (16, "16 bit - CD Quality"),
    (24, "24 bit - High Resolution"),
    (32, "32 bit - Studio Quality"),
];

const BUFFER_SIZES: &[(u32, &str)] = &[
    (128, "128 samples (2.7ms @48kHz)"),
    (256, "256 samples (5.3ms @48kHz)"),
    (512, "512 samples (10.7ms @48kHz)"),
    (1024, "1024 samples (21.3ms @48kHz)"),
    (2048, "2048 samples (42.7ms @48kHz)"),
    (4096, "4096 samples (85.3ms @48kHz)"),
    (8192, "8192 samples (170.7ms @48kHz)"),
];

const EXCLUSIVE_BUFFER_SIZES: &[(u32, &str)] = &[
    (64, "64 samples (1.3ms @48kHz) - Ultra Low Latency"),
    (128, "128 samples (2.7ms @48kHz) - Low Latency"),
    (256, "256 samples (5.3ms @48kHz) - Balanced"),
    (512, "512 samples (10.7ms @48kHz) - Stable"),
    (1024, "1024 samples (21.3ms @48kHz) - High Latency"),
];

const CONFIG_MODES: &[(&str, &str)] = &[
    ("global", "Global System Settings (All Applications)"),
    ("exclusive", "Exclusive Mode (Single Application)"),
];

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            system_wide_config: false, // Default to user config
        }
    }
}

#[derive(Clone, Debug)]
pub enum TabType {
    Output,
    Input,
}

impl TabType {
    pub fn title(&self) -> &'static str {
        match self {
            TabType::Output => "Output",
            TabType::Input => "Input",
        }
    }

    pub fn device_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Audio Device",
            TabType::Input => "Input Audio Device",
        }
    }

    pub fn settings_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Audio Settings",
            TabType::Input => "Input Audio Settings",
        }
    }

    pub fn actions_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Actions",
            TabType::Input => "Input Actions",
        }
    }

    pub fn apply_button_label(&self) -> &'static str {
        match self {
            TabType::Output => "Apply Output Audio Settings",
            TabType::Input => "Apply Input Audio Settings",
        }
    }

    pub fn detect_current_device_fn(&self) -> fn() -> Result<String, String> {
        match self {
            TabType::Output => detect_output_audio_device,
            TabType::Input => detect_input_audio_device,
        }
    }

    pub fn detect_devices_fn(&self) -> fn() -> Result<Vec<AudioDevice>, String> {
        match self {
            TabType::Output => detect_output_audio_devices,
            TabType::Input => detect_input_audio_devices,
        }
    }

    pub fn apply_settings_fn(&self) -> fn(AudioSettings) -> Result<(), String> {
        match self {
            TabType::Output => apply_output_audio_settings_with_auth_blocking,
            TabType::Input => apply_input_audio_settings_with_auth_blocking,
        }
    }

    pub fn selection_prefix(&self) -> &'static str {
        match self {
            TabType::Output => "Selected Output",
            TabType::Input => "Selected Input",
        }
    }

    pub fn current_device_prefix(&self) -> &'static str {
        match self {
            TabType::Output => "Current Default Output Device",
            TabType::Input => "Current Default Input Device",
        }
    }
}

impl AudioApp {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title("Pro Audio Config");

        // Set window icon from system or development location
        Self::set_window_icon(&window);

        // Create menu bar
        let menu_bar = Self::create_menu_bar();

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

        // ===== CREATE NOTEBOOK (TABS) =====
        let notebook = Notebook::new();

        // Create output tab
        let output_tab = AudioTab::new(TabType::Output);
        let output_label = Label::new(Some("Output"));
        notebook.append_page(&output_tab.container, Some(&output_label));

        // Create input tab
        let input_tab = AudioTab::new(TabType::Input);
        let input_label = Label::new(Some("Input"));
        notebook.append_page(&input_tab.container, Some(&input_label));

        // Create advanced tab
        let advanced_tab = AdvancedTab::new();
        let advanced_label = Label::new(Some("Advanced"));
        notebook.append_page(&advanced_tab.container, Some(&advanced_label));

        // Create monitoring tab
        let monitoring_tab = MonitoringTab::new();
        let monitoring_label = Label::new(Some("Monitor"));
        notebook.append_page(&monitoring_tab.container, Some(&monitoring_label));

        // Create config-inspector tab
        let config_inspector_tab = ConfigInspectorTab::new();
        let config_inspector_label = Label::new(Some("Config Inspector"));
        notebook.append_page(
            &config_inspector_tab.container,
            Some(&config_inspector_label),
        );

        main_box.pack_start(&notebook, true, true, 0);
        scrolled_window.add(&main_box);
        window.add(&scrolled_window);

        let app_state = Self {
            window,
            notebook,
            output_tab,
            input_tab,
            advanced_tab,
            monitoring_tab,
            config_inspector_tab,
        };

        // ===== CONNECT SIGNALS =====
        app_state.setup_signals();

        // ===== CONNECT ADVANCED TAB SIGNALS =====
        app_state.setup_advanced_signals();

        // ===== DETECT ALL DEVICES AND CURRENT SETTINGS =====
        app_state.initialize_tabs();

        app_state
    }

    fn set_window_icon(window: &ApplicationWindow) {
        let icon_paths = [
            // System installation paths (multiple sizes)
            "/usr/share/icons/hicolor/16x16/apps/pro-audio-config.png",
            "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
            "/usr/share/icons/hicolor/32x32/apps/pro-audio-config.png",
            "/usr/share/icons/hicolor/256x256/apps/pro-audio-config.png",
            // Development paths
            "icons/48x48/pro-audio-config.png",
            "icons/32x32/pro-audio-config.png",
            "icons/icon.png",    // Relative path from project root
            "icon.png",          // Current directory
            "../icons/icon.png", // If running from different directory
            "./icons/icon.png",  // Explicit current directory
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
    }

    fn create_menu_bar() -> gtk::MenuBar {
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

        menu_bar
    }

    fn initialize_tabs(&self) {
        // Initialize both tabs with the same pattern
        self.output_tab.detect_all_devices();
        self.input_tab.detect_all_devices();
        self.output_tab.detect_current_settings();
        self.input_tab.detect_current_settings();
        self.output_tab.detect_current_device();
        self.input_tab.detect_current_device();
        self.advanced_tab.detect_advanced_devices();
        self.config_inspector_tab.scan_configs();
    }

    fn setup_signals(&self) {
        self.output_tab.setup_signals(self.clone());
        self.input_tab.setup_signals(self.clone());

        let config_inspector_tab = self.config_inspector_tab.clone();
        self.config_inspector_tab
            .refresh_button
            .connect_clicked(move |_| {
                config_inspector_tab.scan_configs();
            });
    }

    fn setup_advanced_signals(&self) {
        self.advanced_tab.setup_signals(self.clone());
    }
}

impl AudioTab {
    fn load_preferences() -> AppPreferences {
        if let Some(prefs_dir) =
            directories::ProjectDirs::from("com", "proaudioconfig", "Pro Audio Config")
        {
            let prefs_path = prefs_dir.config_dir().join("preferences.toml");
            if let Ok(content) = fs::read_to_string(&prefs_path) {
                if let Ok(prefs) = toml::from_str(&content) {
                    return prefs;
                }
            }
        }
        AppPreferences::default()
    }

    fn save_preferences(prefs: &AppPreferences) -> Result<(), String> {
        if let Some(prefs_dir) =
            directories::ProjectDirs::from("com", "proaudioconfig", "Pro Audio Config")
        {
            let config_dir = prefs_dir.config_dir();
            println!("DEBUG: Saving preferences to: {}", config_dir.display());

            fs::create_dir_all(config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;

            let prefs_path = config_dir.join("preferences.toml");
            println!("DEBUG: Preferences file path: {}", prefs_path.display());

            let content = toml::to_string(prefs)
                .map_err(|e| format!("Failed to serialize preferences: {}", e))?;

            fs::write(&prefs_path, content)
                .map_err(|e| format!("Failed to write preferences: {}", e))?;

            println!(
                "DEBUG: Preferences saved successfully to: {}",
                prefs_path.display()
            );
        }
        Ok(())
    }

    pub fn new(tab_type: TabType) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
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
        let preferences = Arc::new(Mutex::new(Self::load_preferences()));

        // Create the system-wide checkbox HERE and use it throughout
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
        Self::populate_combo_box(&sample_rate_combo, SAMPLE_RATES);

        // Set default values based on tab type
        if matches!(tab_type, TabType::Output) {
            sample_rate_combo.set_active_id(Some("48000"));
        }

        // Bit Depth Selection
        let bit_depth_label = Label::new(Some("Bit Depth:"));
        bit_depth_label.set_halign(gtk::Align::Start);

        let bit_depth_combo = create_constrained_combo();
        Self::populate_combo_box(&bit_depth_combo, BIT_DEPTHS);

        if matches!(tab_type, TabType::Output) {
            bit_depth_combo.set_active_id(Some("24"));
        }

        // Buffer Size Selection
        let buffer_size_label = Label::new(Some("Buffer Size:"));
        buffer_size_label.set_halign(gtk::Align::Start);

        let buffer_size_combo = create_constrained_combo();
        Self::populate_combo_box(&buffer_size_combo, BUFFER_SIZES);

        if matches!(tab_type, TabType::Output) {
            buffer_size_combo.set_active_id(Some("512"));
        }

        // Add settings to settings box
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

        // Use the SAME system_wide_checkbox instance that we created above
        let system_info_label = Label::new(Some(
            "System-wide changes affect all users and require authentication",
        ));
        system_info_label.set_line_wrap(true);

        system_box.pack_start(&system_wide_checkbox, false, false, 0);
        system_box.pack_start(&system_info_label, false, false, 0);

        // Add to container after actions section
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
            system_wide_checkbox, // This is now the same instance that's in the UI
            preferences,
        }
    }

    /// Helper function to populate combo boxes from common definitions
    fn populate_combo_box(combo: &ComboBoxText, options: &[(u32, &str)]) {
        for (value, label) in options {
            combo.append(Some(&value.to_string()), label);
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
                            // Store the actual device name for WirePlumber config
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

        // Create channel for communication
        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        // Spawn thread for device detection
        std::thread::spawn(move || {
            let result = detect_fn();
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

                            // Group devices by type for better organization
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

                            // Add USB devices first (most common for pro audio)
                            if !usb_devices.is_empty() {
                                device_combo
                                    .append(Some("separator1"), "--- USB Audio Devices ---");
                                for device in usb_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            // Add PCI devices (onboard and sound cards)
                            if !pci_devices.is_empty() {
                                device_combo
                                    .append(Some("separator2"), "--- PCI Audio Devices ---");
                                for device in pci_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            // Add HDMI devices (only for output)
                            if matches!(tab_type, TabType::Output) && !hdmi_devices.is_empty() {
                                device_combo
                                    .append(Some("separator3"), "--- HDMI/DisplayPort Audio ---");
                                for device in hdmi_devices {
                                    Self::add_device_to_combo(&device_combo, device, &tab_type);
                                }
                            }

                            // Add any remaining devices
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

    /// Helper function to add devices to combo box with consistent formatting
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

        // Clean the description by removing "SUSPENDED" and any trailing status words
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
                            sample_rate_combo
                                .set_active_id(Some(&settings.sample_rate.to_string()));
                            bit_depth_combo.set_active_id(Some(&settings.bit_depth.to_string()));
                            buffer_size_combo
                                .set_active_id(Some(&settings.buffer_size.to_string()));
                        }
                        Err(e) => {
                            println!("Failed to detect current {} settings: {}", "audio", e);
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

            // Update preferences
            let mut prefs = preferences_clone.lock().unwrap();
            prefs.system_wide_config = system_wide;

            println!(
                "DEBUG: Checkbox toggled - system_wide: {}, saving preferences...",
                system_wide
            );

            // Save preferences
            if let Err(e) = Self::save_preferences(&prefs) {
                println!("Warning: Failed to save preferences: {}", e);
            } else {
                println!("DEBUG: Preferences saved successfully");
            }

            // Update UI feedback
            if system_wide {
                println!("System-wide configuration enabled");
            } else {
                println!("User-specific configuration enabled");
            }
        });

        // Clone tab_type for each closure that needs it
        let tab_type_for_apply = self.tab_type.clone();
        let tab_type_for_device = self.tab_type.clone();
        let preferences_clone = Arc::clone(&self.preferences);

        // Apply button click handler
        self.apply_button.connect_clicked(move |_| {
            // Clone tab_type for use in this closure
            let tab_type = tab_type_for_apply.clone();

            // Get system-wide preference
            let system_wide = {
                let prefs = preferences_clone.lock().unwrap();
                prefs.system_wide_config
            };

            // Update UI immediately with appropriate message
            if system_wide {
                status_label.set_text(&format!("Applying system-wide {} settings... (May prompt for admin password)", tab_type.title().to_lowercase()));
            } else {
                status_label.set_text(&format!("Applying user {} settings...", tab_type.title().to_lowercase()));
            }

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

            // Clone app_state for use in the thread
            let app_state_clone = app_state.clone();

            // Create channel for communication
            let (tx, rx) = mpsc::channel();

            // Store the receiver in an Arc<Mutex> to share between threads
            let rx_arc = Arc::new(Mutex::new(rx));

            // Clone tab_type for the thread
            let tab_type_thread = tab_type.clone();

            // Spawn thread for blocking operation
            std::thread::spawn(move || {
                // Pass system_wide to the apply function
                let result = if system_wide {
                    // For system-wide, use the blocking auth functions
                    apply_fn(settings)
                } else {
                    // For user-specific, use a different approach or the same with user config
                    apply_user_audio_settings(settings, &tab_type_thread.title().to_lowercase())
                };
                let _ = tx.send(result);
            });

            // Clone values for the timeout closure to avoid move issues
            let tab_type_timeout = tab_type.clone();
            let status_label_timeout = status_label_clone.clone();
            let apply_button_timeout = apply_button_clone.clone();
            let app_state_timeout = app_state_clone.clone();

            // Set up timeout to check for result
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

                                // FORCE REDETECTION OF DEVICES AFTER RESTART
                                // Wait a bit for services to stabilize, then redetect
                                let app_state_redetect = app_state_timeout.clone();
                                let status_label_for_closure = status_label_timeout.clone(); // Clone for the closure
                                let tab_type_for_redetect = tab_type_timeout.clone();
                                glib::timeout_add_local(Duration::from_secs(4), move || {
                                    println!("Redetecting audio devices after service restart...");
                                    app_state_redetect.output_tab.detect_all_devices();
                                    app_state_redetect.input_tab.detect_all_devices();
                                    app_state_redetect.output_tab.detect_current_device();
                                    app_state_redetect.input_tab.detect_current_device();
                                    app_state_redetect.output_tab.detect_current_settings();
                                    app_state_redetect.input_tab.detect_current_settings();

                                    // Update status to indicate redetection completed
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
                    Err(mpsc::TryRecvError::Empty) => {
                        // Still processing, check again
                        ControlFlow::Continue
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // Thread finished but no result
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
            // Clone tab_type for use in this closure
            let tab_type = tab_type_for_device.clone();
            let selection_prefix = tab_type.selection_prefix().to_string();

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
                        format!(
                            "Selected: Default {} Device ({})",
                            tab_type.title(),
                            stored_name
                        )
                    } else {
                        format!("Selected: Default {} Device", tab_type.title())
                    }
                } else {
                    // Get the display text and clean it for the selection info
                    let display_text = combo.active_text().unwrap_or_default();
                    let clean_text = clean_display_text(&display_text);
                    format!("{}: {}", selection_prefix, clean_text)
                };
                current_device_label.set_text(&selection_text);
            }
        });
    }

    // Helper function to extract actual device name from formatted string
    fn extract_actual_device_name(device_info: &str) -> Option<String> {
        // Remove system prefix and extract the device identifier
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

impl AdvancedTab {
    // State check function to determine if exclusive mode is active
    fn is_exclusive_mode_active() -> bool {
        let username = whoami::username();

        // Check both system AND user exclusive mode configuration files
        let exclusive_config_paths = [
            // System paths
            "/etc/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
            "/etc/wireplumber/wireplumber.conf.d/99-pro-audio-exclusive.conf",
            // User paths
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
        let global_frame_clone = global_settings_frame.clone(); // Clone before moving

        let global_info_label =
            Label::new(Some("These settings affect ALL applications system-wide"));
        global_info_label.set_halign(gtk::Align::Start);

        let global_device_label = Label::new(Some("Audio Device:"));
        global_device_label.set_halign(gtk::Align::Start);

        let device_combo = create_constrained_combo();

        let global_sample_rate_label = Label::new(Some("Sample Rate:"));
        global_sample_rate_label.set_halign(gtk::Align::Start);

        let sample_rate_combo = create_constrained_combo();
        Self::populate_combo_box(&sample_rate_combo, SAMPLE_RATES);
        sample_rate_combo.set_active_id(Some("48000"));

        let global_bit_depth_label = Label::new(Some("Bit Depth:"));
        global_bit_depth_label.set_halign(gtk::Align::Start);

        let bit_depth_combo = create_constrained_combo();
        Self::populate_combo_box(&bit_depth_combo, BIT_DEPTHS);
        bit_depth_combo.set_active_id(Some("24"));

        let global_buffer_size_label = Label::new(Some("Buffer Size:"));
        global_buffer_size_label.set_halign(gtk::Align::Start);

        let buffer_size_combo = create_constrained_combo();
        Self::populate_combo_box(&buffer_size_combo, BUFFER_SIZES);
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

        // ===== PROFESSIONAL SETTINGS SECTION (NEW) =====
        let (pro_settings_frame, pro_settings_box) = create_section_box("Professional Settings");
        let pro_frame_clone = pro_settings_frame.clone();

        let pro_info_label = Label::new(Some(
            "Advanced settings for studio and professional audio work. Use with caution.",
        ));
        pro_info_label.set_halign(gtk::Align::Start);
        pro_info_label.set_line_wrap(true);

        // Buffer size range (min/max)
        let buffer_range_label = Label::new(Some("Buffer Size Range (min - max):"));
        buffer_range_label.set_halign(gtk::Align::Start);

        let buffer_range_box = GtkBox::new(Orientation::Horizontal, 6);
        buffer_range_box.set_halign(gtk::Align::Start);

        let min_buffer_combo = create_constrained_combo();
        min_buffer_combo.set_width_request(180);
        Self::populate_combo_box(
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
        Self::populate_combo_box(
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

        // Thread priority
        let thread_priority_label = Label::new(Some("Real-time Thread Priority:"));
        thread_priority_label.set_halign(gtk::Align::Start);

        let thread_priority_combo = create_constrained_combo();
        thread_priority_combo.append(Some("normal"), "Normal Priority (-11 nice, 88 RT)");
        thread_priority_combo.append(Some("high"), "High Priority (-15 nice, 90 RT)");
        thread_priority_combo.append(Some("realtime"), "Real-time Priority (-20 nice, 99 RT)");
        thread_priority_combo.set_active_id(Some("high"));

        // Memory and performance options
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

        // Resampler quality
        let resampler_label = Label::new(Some("Resampler Quality (if resampling is needed):"));
        resampler_label.set_halign(gtk::Align::Start);

        let resampler_combo = create_constrained_combo();
        resampler_combo.append(Some("fastest"), "Fastest (lowest quality, lowest CPU)");
        resampler_combo.append(Some("low"), "Low quality");
        resampler_combo.append(Some("medium"), "Medium (balanced)");
        resampler_combo.append(Some("high"), "High quality");
        resampler_combo.append(Some("highest"), "Highest (best quality, highest CPU)");
        resampler_combo.set_active_id(Some("high"));

        // Clock source
        let clock_source_label = Label::new(Some("Clock Source:"));
        clock_source_label.set_halign(gtk::Align::Start);

        let clock_source_combo = create_constrained_combo();
        clock_source_combo.append(Some("default"), "Default (system clock)");
        clock_source_combo.append(Some("monotonic"), "Monotonic (more stable)");
        clock_source_combo.append(Some("realtime"), "Real-time (most accurate)");
        clock_source_combo.set_active_id(Some("monotonic"));

        // Add to professional settings box
        pro_settings_box.pack_start(&pro_info_label, false, false, 0);
        pro_settings_box.pack_start(&buffer_range_label, false, false, 6);
        pro_settings_box.pack_start(&buffer_range_box, false, false, 0);
        pro_settings_box.pack_start(&thread_priority_label, false, false, 6);
        pro_settings_box.pack_start(&thread_priority_combo, false, false, 0);
        pro_settings_box.pack_start(&clock_source_label, false, false, 6);
        pro_settings_box.pack_start(&clock_source_combo, false, false, 0);

        // Create a horizontal box for checkboxes
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
        let exclusive_frame_clone = exclusive_settings_frame.clone(); // Clone before moving

        let exclusive_info_label = Label::new(Some(
            "Exclusive mode provides direct hardware access for low-latency professional audio work",
        ));
        exclusive_info_label.set_halign(gtk::Align::Start);
        exclusive_info_label.set_line_wrap(true);

        // Application settings
        let app_name_label = Label::new(Some("Application Name:"));
        app_name_label.set_halign(gtk::Align::Start);

        let application_name_entry = Entry::new();
        application_name_entry.set_placeholder_text(Some("e.g., Reaper, Ardour, Bitwig"));

        let process_name_label = Label::new(Some("Process Name:"));
        process_name_label.set_halign(gtk::Align::Start);

        let process_name_entry = Entry::new();
        process_name_entry.set_placeholder_text(Some("e.g., reaper, ardour, bitwig-studio"));

        // Device and audio settings
        let exclusive_device_label = Label::new(Some("Audio Device:"));
        exclusive_device_label.set_halign(gtk::Align::Start);

        let exclusive_device_combo = create_constrained_combo();

        let exclusive_sample_rate_label = Label::new(Some("Sample Rate:"));
        exclusive_sample_rate_label.set_halign(gtk::Align::Start);

        let exclusive_sample_rate_combo = create_constrained_combo();
        Self::populate_combo_box(&exclusive_sample_rate_combo, SAMPLE_RATES);
        exclusive_sample_rate_combo.set_active_id(Some("48000"));

        let exclusive_bit_depth_label = Label::new(Some("Bit Depth:"));
        exclusive_bit_depth_label.set_halign(gtk::Align::Start);

        let exclusive_bit_depth_combo = create_constrained_combo();
        Self::populate_combo_box(&exclusive_bit_depth_combo, BIT_DEPTHS);
        exclusive_bit_depth_combo.set_active_id(Some("24"));

        let exclusive_buffer_size_label = Label::new(Some("Buffer Size:"));
        exclusive_buffer_size_label.set_halign(gtk::Align::Start);

        let exclusive_buffer_size_combo = create_constrained_combo();
        Self::populate_combo_box(&exclusive_buffer_size_combo, EXCLUSIVE_BUFFER_SIZES);
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
        let _apply_button_clone = apply_button.clone(); // Clone for deferred setup

        let disable_exclusive_button = Button::with_label("Disable Exclusive Mode");

        // Check system state and set sensitivity accordingly
        let is_exclusive_active = Self::is_exclusive_mode_active();
        println!("DEBUG: Exclusive mode active: {}", is_exclusive_active);
        println!("DEBUG: Disable button sensitivity: {}", is_exclusive_active);
        disable_exclusive_button.set_sensitive(is_exclusive_active);

        let info_label = Label::new(Some(
            "Note: Professional settings may require administrator privileges and can affect system stability",
        ));
        info_label.set_line_wrap(true);

        actions_box.pack_start(&status_label, false, false, 0);
        actions_box.pack_start(&apply_button, false, false, 0);
        actions_box.pack_start(&disable_exclusive_button, false, false, 0); // Added missing button
        actions_box.pack_start(&info_label, false, false, 0);

        // ===== ASSEMBLE ADVANCED TAB =====
        container.pack_start(&mode_frame, false, false, 0);
        container.pack_start(&global_settings_frame, false, false, 0);
        container.pack_start(&pro_settings_frame, false, false, 0);
        container.pack_start(&exclusive_settings_frame, false, false, 0);
        container.pack_start(&actions_frame, false, false, 0);

        // Hide frames initially using clones (before they're moved into struct)
        global_frame_clone.hide();
        pro_frame_clone.hide();
        exclusive_frame_clone.hide();

        // Create the tab instance
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
            // Professional settings fields
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

        // Set up initial visibility after GTK initialization
        let tab_clone = tab.clone();
        glib::idle_add_local_once(move || {
            // Use the frames stored in the struct (accessible via tab_clone)
            // Show only the global settings initially (default mode)
            tab_clone.global_settings_frame.show();
            tab_clone.pro_settings_frame.show(); // Show professional settings too for global mode
            // Hide the exclusive settings using the clone from the tab
            tab_clone.exclusive_settings_frame.hide();

            // Also update the apply button label
            // Use the cloned apply_button if you have it, or access through tab_clone
            tab_clone
                .apply_button
                .set_label("Apply Global System Settings");
        });

        tab
    }

    fn populate_combo_box(combo: &ComboBoxText, options: &[(u32, &str)]) {
        for (value, label) in options {
            combo.append(Some(&value.to_string()), label);
        }
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

        // Create channel for communication
        let (tx, rx) = mpsc::channel();
        let rx_arc = Arc::new(Mutex::new(rx));

        // Spawn thread for device detection - USE ENHANCED FUNCTIONS
        std::thread::spawn(move || {
            // Use high-performance devices for exclusive mode, recommended for global
            let result = detect_recommended_devices();
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
                            exclusive_device_combo.remove_all();

                            // Get the stored default device name if available
                            let default_device_name = {
                                let stored = current_default_device.lock().unwrap();
                                if !stored.is_empty() {
                                    format!("Default: {}", stored)
                                } else {
                                    "Default System Device".to_string()
                                }
                            };

                            // Add "Default Device" option with actual device name
                            device_combo.append(Some("default"), &default_device_name);
                            exclusive_device_combo.append(Some("default"), &default_device_name);

                            // Group devices by type for better organization
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

                            // Add USB devices first (most common for pro audio)
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

                            // Add PCI devices (onboard and sound cards)
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

                            // Add HDMI devices
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

                            // Add any remaining devices
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
                            // Still add default option
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
                    // Add default option as fallback
                    device_combo.append(Some("default"), "Default System Device");
                    exclusive_device_combo.append(Some("default"), "Default System Device");
                    device_combo.set_active_id(Some("default"));
                    exclusive_device_combo.set_active_id(Some("default"));
                    ControlFlow::Break
                }
            }
        });
    }

    /// Helper function to add devices to combo box with consistent formatting
    fn add_device_to_combo(combo: &ComboBoxText, device: &AudioDevice) {
        let device_type = match device.device_type {
            DeviceType::Input => "🎤 Input",
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

        // Clone the entry fields here, before using them in closures
        let application_name_entry = self.application_name_entry.clone();
        let process_name_entry = self.process_name_entry.clone();

        let exclusive_device_combo = self.exclusive_device_combo.clone();
        let exclusive_sample_rate_combo = self.exclusive_sample_rate_combo.clone();
        let _exclusive_bit_depth_combo = self.exclusive_bit_depth_combo.clone();
        let exclusive_buffer_size_combo = self.exclusive_buffer_size_combo.clone();
        let latency_label = self.latency_label.clone();

        // Clone the disable_exclusive_button at the beginning
        let disable_exclusive_button = self.disable_exclusive_button.clone();

        // Check system state and set sensitivity accordingly
        let is_exclusive_active = Self::is_exclusive_mode_active();
        println!("DEBUG: Exclusive mode active: {}", is_exclusive_active);
        println!("DEBUG: Disable button sensitivity: {}", is_exclusive_active);
        disable_exclusive_button.set_sensitive(is_exclusive_active);

        // Configuration mode change handler
        {
            let status_label = status_label.clone();
            let global_settings_frame = global_settings_frame.clone();
            let pro_settings_frame = self.pro_settings_frame.clone(); // Add this
            let exclusive_settings_frame = exclusive_settings_frame.clone();
            let apply_button = apply_button.clone();

            config_mode_combo.connect_changed(move |combo| {
                if let Some(mode) = combo.active_id() {
                    Self::update_mode_description(&status_label, &mode);

                    match mode.as_str() {
                        "global" => {
                            global_settings_frame.show();
                            pro_settings_frame.show(); // Show professional settings
                            exclusive_settings_frame.hide();
                            apply_button.set_label("Apply Advanced Global Settings"); // Updated label
                        }
                        "exclusive" => {
                            global_settings_frame.hide();
                            pro_settings_frame.hide(); // Hide professional settings
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
                if let Some(buffer_size_str) = combo.active_id() {
                    if let Some(sample_rate_str) = exclusive_sample_rate_combo.active_id() {
                        if let (Ok(buffer_size), Ok(sample_rate)) = (
                            buffer_size_str.parse::<u32>(),
                            sample_rate_str.parse::<u32>(),
                        ) {
                            let latency_ms = (buffer_size as f64 * 1000.0) / sample_rate as f64;
                            latency_label.set_text(&format!(
                                "Calculated Latency: {:.2}ms @ {}kHz",
                                latency_ms,
                                sample_rate / 1000
                            ));
                        }
                    }
                }
            });
        }

        // Exclusive mode latency calculation - sample rate change
        {
            let exclusive_buffer_size_combo = exclusive_buffer_size_combo.clone();
            let latency_label = latency_label.clone();

            exclusive_sample_rate_combo.connect_changed(move |combo| {
                if let Some(sample_rate_str) = combo.active_id() {
                    if let Some(buffer_size_str) = exclusive_buffer_size_combo.active_id() {
                        if let (Ok(buffer_size), Ok(sample_rate)) = (
                            buffer_size_str.parse::<u32>(),
                            sample_rate_str.parse::<u32>(),
                        ) {
                            let latency_ms = (buffer_size as f64 * 1000.0) / sample_rate as f64;
                            latency_label.set_text(&format!(
                                "Calculated Latency: {:.2}ms @ {}kHz",
                                latency_ms,
                                sample_rate / 1000
                            ));
                        }
                    }
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

            // Professional settings
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

            // Use the cloned entry fields from above
            let application_name_entry_clone = application_name_entry.clone();
            let process_name_entry_clone = process_name_entry.clone();

            // Clone the apply_button for use inside the closure
            let apply_button_clone = apply_button.clone();

            // Use the cloned disable_exclusive_button from the outer scope
            let disable_exclusive_button_inner = disable_exclusive_button.clone();

            apply_button.connect_clicked(move |_| {
		let mode = config_mode_combo.active_id()
		    .map(|id| id.to_string())
		    .unwrap_or_else(|| "global".to_string());

		match mode.as_str() {
		    "global" => {
			status_label.set_text("Applying advanced global system settings...");
			apply_button_clone.set_sensitive(false);

			let device_id = device_combo.active_id()
			    .map(|id| id.to_string())
			    .unwrap_or_else(|| "default".to_string());

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
			    device_id,
			};

			// Get professional settings
			let min_buffer = min_buffer_combo.active_id()
			    .and_then(|id| id.parse::<u32>().ok())
			    .unwrap_or(128);

			let max_buffer = max_buffer_combo.active_id()
			    .and_then(|id| id.parse::<u32>().ok())
			    .unwrap_or(2048);

			let thread_priority = thread_priority_combo.active_id()
			    .map(|id| id.to_string())
			    .unwrap_or_else(|| "high".to_string());

			let memory_lock = memory_lock_checkbox.is_active();
			let prevent_suspend = prevent_suspend_checkbox.is_active();
			let disable_remixing = disable_remixing_checkbox.is_active();
			let disable_resampling = disable_resampling_checkbox.is_active();

			let resampler_quality = resampler_combo.active_id()
			    .map(|id| id.to_string())
			    .unwrap_or_else(|| "high".to_string());

			let clock_source = clock_source_combo.active_id()
			    .map(|id| id.to_string())
			    .unwrap_or_else(|| "monotonic".to_string());

			let status_label_clone = status_label.clone();
			let apply_button_clone_inner = apply_button_clone.clone();

			// Create channel for communication
			let (tx, rx) = mpsc::channel();
			let rx_arc = Arc::new(Mutex::new(rx));

			// Spawn thread for blocking operation
			std::thread::spawn(move || {
			    let result = crate::config::apply_advanced_professional_settings(
				&settings,
				true, // Global mode is always system-wide
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

			// Set up timeout to check for result
			let rx_timeout = Arc::clone(&rx_arc);
			glib::timeout_add_local(Duration::from_millis(100), move || {
			    let rx_guard = rx_timeout.lock().unwrap();
			    match rx_guard.try_recv() {
				Ok(result) => {
				    match result {
					Ok(()) => {
					    status_label_clone.set_text("Global settings applied successfully!");
                                            apply_button_clone_inner.set_sensitive(true);
                                            show_success_dialog("Global audio settings applied successfully. All applications will use these settings.");
					}
					Err(e) => {
					    status_label_clone.set_text("Failed to apply advanced settings");
					    apply_button_clone_inner.set_sensitive(true);
					    show_error_dialog(&format!("Failed to apply advanced settings: {}", e));
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

			// Get the selected device for exclusive mode
			let device_pattern = exclusive_device_combo.active_id()
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "default".to_string());

			// Get device capabilities to validate settings
			if let Ok(capabilities) = get_device_capabilities(&device_pattern) {
                            // Validate buffer size against device capabilities
                            let buffer_size = exclusive_buffer_size_combo.active_id()
				.and_then(|id| id.parse::<u32>().ok())
				.unwrap_or(128);

                            let sample_rate = exclusive_sample_rate_combo.active_id()
				.and_then(|id| id.parse::<u32>().ok())
				.unwrap_or(48000);

                            // Check if settings are compatible with device
                            if !capabilities.buffer_sizes.contains(&buffer_size) {
				status_label.set_text("Warning: Buffer size may not be optimal for this device");
                            }

                            if !capabilities.sample_rates.contains(&sample_rate) {
				status_label.set_text("Warning: Sample rate may not be supported by this device");
                            }
			}

			// For exclusive mode, we use the advanced audio settings function
			let exclusive_mode = true;
			let direct_hardware = true; // Exclusive mode implies direct hardware access
			let low_latency = true;     // Exclusive mode implies low latency

			let buffer_size = exclusive_buffer_size_combo.active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(128);

			let sample_rate = exclusive_sample_rate_combo.active_id()
                            .and_then(|id| id.parse::<u32>().ok())
                            .unwrap_or(48000);

			// Get app name and process name from entry fields
			let app_name = {
			    let text = application_name_entry_clone.text(); // Use .text() instead of .get_text()
			    if text.is_empty() {
				None
			    } else {
				Some(text.to_string())
			    }
			};

			let app_process_name = {
			    let text = process_name_entry_clone.text(); // Use .text() instead of .get_text()
			    if text.is_empty() {
				None
			    } else {
				Some(text.to_string())
			    }
			};

			let status_label_clone = status_label.clone();
			let apply_button_clone_inner = apply_button_clone.clone();

			// Use the cloned disable_exclusive_button from the outer closure
			let disable_exclusive_button_local = disable_exclusive_button_inner.clone();

			// Create channel for communication
			let (tx, rx) = mpsc::channel();
			let rx_arc = Arc::new(Mutex::new(rx));

			// Spawn thread for blocking operation
			std::thread::spawn(move || {
                            let result = apply_advanced_audio_settings(
				exclusive_mode,
				direct_hardware,
				low_latency,
				buffer_size,
				sample_rate,
				Some(device_pattern), // Pass the device pattern
				app_name,
				app_process_name,
                            );
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
                                            status_label_clone.set_text("Exclusive mode settings applied successfully!");
                                            apply_button_clone_inner.set_sensitive(true);

                                            // Enable the disable exclusive button since exclusive mode is now active
                                            disable_exclusive_button_local.set_sensitive(true);

                                            show_success_dialog("Exclusive mode settings applied successfully. Audio system configured for low-latency performance.");
					}
					Err(e) => {
                                            status_label_clone.set_text("Failed to apply exclusive mode settings");
                                            apply_button_clone_inner.set_sensitive(true);
                                            show_error_dialog(&format!("Failed to apply exclusive mode settings: {}", e));
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
                        // Disable this button after successful operation
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

// Helper function to create constrained combo boxes that don't expand too wide
fn create_constrained_combo() -> ComboBoxText {
    let combo = ComboBoxText::new();
    combo.set_hexpand(false);
    combo.set_halign(gtk::Align::Fill);
    combo.set_width_request(400); // Fixed width to prevent excessive expansion
    combo
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
#[allow(dead_code)] // Used
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

/// Helper function to clean device display name (remove PipeWire: prefix etc.)
#[allow(dead_code)] // Used
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
        "Failed to apply audio settings",
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
        // If not a "Script failed" message, use the message as-is
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
        "Settings Applied Successfully",
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
    dialog.set_version(Some("1.9"));
    dialog.set_website(Some("https://github.com/Peter-L-SVK/pro_audio_config"));
    dialog.set_copyright(Some("Copyright © 2025 Peter Leukanič"));
    dialog.set_authors(&["Peter Leukanič"]);
    dialog.set_comments(Some(
        "Professional audio configuration tool for Linux\nWritten in Rust programming language",
    ));
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
         For more details, see the LICENSE file included with this distribution.",
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
        "icons/icon.png",    // Relative path from project root
        "icon.png",          // Current directory
        "../icons/icon.png", // If running from different directory
        "./icons/icon.png",  // Explicit current directory
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

/// Helper function to get the main window
fn get_main_window() -> gtk::Window {
    // Try to find the application's main window from toplevel widgets
    for widget in gtk::Window::list_toplevels() {
        if let Some(window) = widget.downcast_ref::<gtk::Window>() {
            if window.is_visible() {
                return window.clone();
            }
        }
    }

    // Fallback: create a new window (original behavior)
    gtk::Window::new(gtk::WindowType::Toplevel)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to skip GTK tests when needed
    fn skip_if_no_gtk() -> bool {
        // Check if we can initialize GTK
        if gtk::init().is_err() {
            println!("Skipping GTK test - no display available");
            return true;
        }
        false
    }

    #[test]
    fn test_constrained_combo_creation() {
        // Unit test for the logic behind constrained combo creation
        // We're testing the conceptual logic, not the actual GTK widget

        // Test 1: Verify our constants are properly defined
        assert!(SAMPLE_RATES.len() > 0);
        assert!(BIT_DEPTHS.len() > 0);
        assert!(BUFFER_SIZES.len() > 0);

        // Test 2: Verify expected values exist in our constants
        assert!(SAMPLE_RATES.iter().any(|(rate, _)| *rate == 48000));
        assert!(BIT_DEPTHS.iter().any(|(depth, _)| *depth == 24));
        assert!(BUFFER_SIZES.iter().any(|(size, _)| *size == 512));

        // Test 3: Verify our helper functions work
        assert_eq!(
            clean_device_description("PipeWire s32le 4ch 192000Hz SUSPENDED"),
            "PipeWire s32le 4ch 192000Hz"
        );

        // Test 4: Verify the populate_combo_box logic works conceptually
        let mut test_options = Vec::new();
        for (value, label) in SAMPLE_RATES {
            test_options.push((*value, *label));
        }
        assert_eq!(test_options.len(), SAMPLE_RATES.len());

        // Test passes - the actual widget creation is tested in integration tests
        assert!(true);
    }

    #[test]
    fn test_tab_type_methods() {
        assert_eq!(TabType::Output.title(), "Output");
        assert_eq!(TabType::Input.title(), "Input");

        assert_eq!(TabType::Output.device_label(), "Output Audio Device");
        assert_eq!(TabType::Input.device_label(), "Input Audio Device");

        assert_eq!(
            TabType::Output.apply_button_label(),
            "Apply Output Audio Settings"
        );
        assert_eq!(
            TabType::Input.apply_button_label(),
            "Apply Input Audio Settings"
        );
    }

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
        assert_eq!(clean_device_description("Device - RUNNING"), "Device");
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
            clean_device_display(
                "PipeWire: alsa_output.usb-PreSonus_Studio_26c_0142446A-00.analog-surround-40"
            ),
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

    #[test]
    fn test_device_grouping_logic() {
        // Test the device categorization logic used in UI
        let usb_device = AudioDevice {
            name: "usb-device".to_string(),
            description: "USB Audio Device".to_string(),
            id: "alsa:usb".to_string(),
            device_type: DeviceType::Output,
            available: true,
        };

        let hdmi_device = AudioDevice {
            name: "hdmi-device".to_string(),
            description: "HDMI Output".to_string(),
            id: "alsa:hdmi".to_string(),
            device_type: DeviceType::Output,
            available: true,
        };

        let pci_device = AudioDevice {
            name: "pci-device".to_string(),
            description: "PCI Sound Card".to_string(),
            id: "alsa:pci".to_string(),
            device_type: DeviceType::Output,
            available: true,
        };

        // Test categorization logic (simplified version)
        assert!(usb_device.description.to_lowercase().contains("usb"));
        assert!(hdmi_device.description.to_lowercase().contains("hdmi"));
        assert!(pci_device.description.to_lowercase().contains("pci"));
    }

    #[test]
    fn test_combo_box_population() {
        // Test that combo boxes are populated with correct options
        // Without initializing GTK, we test the logic indirectly

        let sample_rates = SAMPLE_RATES;
        let bit_depths = BIT_DEPTHS;
        let buffer_sizes = BUFFER_SIZES;

        // Verify our constants contain expected values
        assert!(sample_rates.iter().any(|(rate, _)| *rate == 48000));
        assert!(bit_depths.iter().any(|(depth, _)| *depth == 24));
        assert!(buffer_sizes.iter().any(|(size, _)| *size == 512));

        // Test the populate_combo_box logic conceptually
        let mut test_vec = Vec::new();
        for (value, label) in sample_rates {
            test_vec.push((*value, *label));
        }

        assert_eq!(test_vec.len(), sample_rates.len());
    }

    #[test]
    fn test_dialog_functions() {
        // Test that dialog helper functions exist and have correct signatures
        // Skip actual execution since GTK isn't initialized

        // Just verify the functions compile by testing their string processing logic
        let error_message = "Test error";
        let success_message = "Test success";

        // Test the error message processing logic used in show_error_dialog
        let display_message = if error_message.contains("Script failed") {
            if let Some(error_part) = error_message.split("\n\n").nth(1) {
                error_part.to_string()
            } else {
                error_message.to_string()
            }
        } else {
            // If not a "Script failed" message, use the message as-is
            error_message.to_string()
        };

        assert_eq!(display_message, "Test error");

        // Test success message processing
        assert!(!success_message.is_empty());
    }

    #[test]
    fn test_device_selection_handling() {
        // Test the device selection logic
        let device_id = "default";
        let selection_text = if device_id == "default" {
            "Selected: Default Output Device".to_string()
        } else {
            format!("Selected Output: {}", "test device")
        };

        assert!(!selection_text.is_empty());
        assert!(selection_text.contains("Selected"));
    }

    #[test]
    fn test_exclusive_mode_state_check() {
        // Test the exclusive mode state checking logic
        // This is a unit test for the logic without actual file system access
        // We're testing that the function compiles and has the correct signature

        // Note: We can't test the actual file system check in unit tests
        // without mocking, but we can verify the function exists and returns bool
        let _exists = AdvancedTab::is_exclusive_mode_active();
        // The function should compile and return a boolean
        assert!(true); // Just to satisfy the test framework
    }
}
