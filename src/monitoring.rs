/*
 * Pro Audio Config - Monitoring Module
 * Version: 1.9
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 *
 * Real-time audio monitoring and configuration display
 */

use crate::audio::{
    clear_cache as clear_audio_cache, detect_audio_system, detect_current_audio_settings,
    detect_input_audio_device, detect_output_audio_device,
};
use glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Frame, Label, Orientation, ProgressBar, Separator};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::audio_capture::PipeWireMonitor;

// Message types for thread communication
#[derive(Debug, Clone)]
enum MonitorMessage {
    Status(String),
    Config {
        sample_rate: u32,
        bit_depth: u32,
        buffer_size: u32,
        audio_system: String,
    },
    Devices {
        output: String,
        input: String,
    },
    Levels {
        left_level: f64,
        left_db: String,
        right_level: f64,
        right_db: String,
    },
    Error(String), // Add error message type
}

#[derive(Clone)]
pub struct MonitoringTab {
    pub container: GtkBox,
    status_label: Label,
    config_info_label: Label,
    device_info_label: Label,
    sample_rate_label: Label,
    bit_depth_label: Label,
    buffer_size_label: Label,
    left_channel_meter: ProgressBar,
    right_channel_meter: ProgressBar,
    system_info_label: Label,
    reconnect_button: Button,
    update_thread_running: Arc<Mutex<bool>>,
    sender: mpsc::Sender<MonitorMessage>,
}

fn manual_pw_link_connection() -> Result<(), String> {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    println!("MANUAL: Listing all monitor ports...");

    // First, list all monitor ports
    let output = Command::new("pw-link")
        .args(["--output"])
        .output()
        .map_err(|e| format!("pw-link failed: {}", e))?;

    if !output.status.success() {
        return Err("pw-link command failed".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut monitor_ports = Vec::new();

    for line in output_str.lines() {
        if line.contains("monitor_") && !line.contains("pro_audio_config") {
            monitor_ports.push(line.trim().to_string());
            println!("MANUAL: Found monitor port: {}", line.trim());
        }
    }

    if monitor_ports.is_empty() {
        return Err("No monitor ports found. Is audio playing?".to_string());
    }

    // Get your app's input ports
    let input_output = Command::new("pw-link")
        .args(["--input"])
        .output()
        .map_err(|e| format!("pw-link --input failed: {}", e))?;

    let input_str = String::from_utf8_lossy(&input_output.stdout);
    let mut input_ports = Vec::new();

    for line in input_str.lines() {
        if line.contains("pro_audio_config:input_") {
            input_ports.push(line.trim().to_string());
            println!("MANUAL: Found input port: {}", line.trim());
        }
    }

    if input_ports.is_empty() {
        return Err("No pro_audio_config input ports found. Is the app running?".to_string());
    }

    // Try to connect matching channels
    let mut connected = 0;
    let mut errors = Vec::new();

    for monitor_port in &monitor_ports {
        // Extract channel name
        if let Some(colon_pos) = monitor_port.rfind(':') {
            let channel_name = &monitor_port[colon_pos + 1..]; // e.g., "monitor_FL"
            let simple_channel = channel_name.replace("monitor_", "");

            // Find matching input port
            let target_port = format!("pro_audio_config:input_{}", simple_channel);

            if input_ports.iter().any(|p| p == &target_port) {
                println!("MANUAL: Connecting {} -> {}", monitor_port, target_port);

                for attempt in 1..=3 {
                    match Command::new("pw-link")
                        .args([monitor_port, &target_port])
                        .status()
                    {
                        Ok(status) if status.success() => {
                            println!("MANUAL: ✓ Connected {} channel", simple_channel);
                            connected += 1;
                            thread::sleep(Duration::from_millis(100));
                            break;
                        }
                        Ok(_) if attempt < 3 => {
                            println!("MANUAL: Retry {}...", attempt);
                            thread::sleep(Duration::from_millis(300));
                        }
                        Err(e) if attempt < 3 => {
                            println!("MANUAL: Error: {}, retrying...", e);
                            thread::sleep(Duration::from_millis(300));
                        }
                        _ => {
                            errors.push(format!("Failed to connect {}", simple_channel));
                        }
                    }
                }
            }
        }
    }

    if connected > 0 {
        println!("MANUAL: Successfully connected {} channels", connected);

        // Verify connections
        thread::sleep(Duration::from_millis(500));

        if let Ok(verify) = Command::new("pw-link").args(["--links"]).output() {
            let verify_str = String::from_utf8_lossy(&verify.stdout);
            let links = verify_str
                .lines()
                .filter(|line| line.contains("pro_audio_config"))
                .count();
            println!("MANUAL: Verified {} active connections", links);
        }

        Ok(())
    } else {
        let all_errors = if errors.is_empty() {
            "No matching channels found".to_string()
        } else {
            errors.join(", ")
        };

        Err(format!("Failed to connect any channels: {}", all_errors))
    }
}

impl MonitoringTab {
    pub fn new() -> Self {
        // Load CSS styles first
        Self::load_css();

        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);

        // ===== STATUS SECTION =====
        let (status_frame, status_box) = create_section_box("Monitoring Status");

        let status_label = Label::new(Some("Starting audio monitoring..."));
        status_label.set_halign(gtk::Align::Start);

        let status_info_label = Label::new(Some(
            "Real-time audio monitoring active. This tab displays current audio configuration and output levels.",
        ));
        status_info_label.set_line_wrap(true);
        status_info_label.set_halign(gtk::Align::Start);

        status_box.pack_start(&status_label, false, false, 0);
        status_box.pack_start(&status_info_label, false, false, 0);

        // ===== CONFIGURATION INFO SECTION =====
        let (config_frame, config_box) = create_section_box("Current Configuration");

        let config_info_label = Label::new(Some("Loading configuration..."));
        config_info_label.set_halign(gtk::Align::Start);
        config_info_label.set_selectable(true);

        let sample_rate_label = Label::new(Some("Sample Rate: --"));
        sample_rate_label.set_halign(gtk::Align::Start);

        let bit_depth_label = Label::new(Some("Bit Depth: --"));
        bit_depth_label.set_halign(gtk::Align::Start);

        let buffer_size_label = Label::new(Some("Buffer Size: --"));
        buffer_size_label.set_halign(gtk::Align::Start);

        let system_info_label = Label::new(Some("Audio System: --"));
        system_info_label.set_halign(gtk::Align::Start);

        config_box.pack_start(&config_info_label, false, false, 0);
        config_box.pack_start(&sample_rate_label, false, false, 0);
        config_box.pack_start(&bit_depth_label, false, false, 0);
        config_box.pack_start(&buffer_size_label, false, false, 0);
        config_box.pack_start(&system_info_label, false, false, 0);

        // ===== DEVICE INFO SECTION =====
        let (device_frame, device_box) = create_section_box("Audio Devices");

        let device_info_label = Label::new(Some("Loading device information..."));
        device_info_label.set_halign(gtk::Align::Start);
        device_info_label.set_line_wrap(true);
        device_info_label.set_selectable(true);

        device_box.pack_start(&device_info_label, false, false, 0);

        // ===== AUDIO METERS SECTION =====
        let (meter_frame, meter_box) = create_section_box("Audio Output Levels");

        let meter_info_label = Label::new(Some("Channel output levels (real-time monitoring):"));
        meter_info_label.set_halign(gtk::Align::Start);

        // Left channel meter
        let left_channel_box = GtkBox::new(Orientation::Horizontal, 6);
        let left_channel_label = Label::new(Some("Left Channel:"));
        left_channel_label.set_width_request(100);

        let left_channel_meter = ProgressBar::new();
        left_channel_meter.set_show_text(true);
        left_channel_meter.set_text(Some("-- dB"));
        left_channel_meter.set_fraction(0.0);
        left_channel_meter.set_width_request(300);

        left_channel_box.pack_start(&left_channel_label, false, false, 0);
        left_channel_box.pack_start(&left_channel_meter, true, true, 0);

        // Right channel meter
        let right_channel_box = GtkBox::new(Orientation::Horizontal, 6);
        let right_channel_label = Label::new(Some("Right Channel:"));
        right_channel_label.set_width_request(100);

        let right_channel_meter = ProgressBar::new();
        right_channel_meter.set_show_text(true);
        right_channel_meter.set_text(Some("-- dB"));
        right_channel_meter.set_fraction(0.0);
        right_channel_meter.set_width_request(300);

        right_channel_box.pack_start(&right_channel_label, false, false, 0);
        right_channel_box.pack_start(&right_channel_meter, true, true, 0);

        // Level indicator key
        let level_key_box = GtkBox::new(Orientation::Horizontal, 12);
        level_key_box.set_halign(gtk::Align::Center);

        let safe_box = GtkBox::new(Orientation::Horizontal, 4);
        let safe_indicator = GtkBox::new(Orientation::Horizontal, 0);
        safe_indicator.set_size_request(16, 16);
        safe_indicator.style_context().add_class("level-safe");
        safe_indicator.style_context().add_class("level-indicator");
        let safe_label = Label::new(Some("Safe (< -3dB)"));
        safe_box.pack_start(&safe_indicator, false, false, 0);
        safe_box.pack_start(&safe_label, false, false, 4);

        let warning_box = GtkBox::new(Orientation::Horizontal, 4);
        let warning_indicator = GtkBox::new(Orientation::Horizontal, 0);
        warning_indicator.set_size_request(16, 16);
        warning_indicator.style_context().add_class("level-warning");
        warning_indicator
            .style_context()
            .add_class("level-indicator");
        let warning_label = Label::new(Some("Warning (-3dB to 0dB)"));
        warning_box.pack_start(&warning_indicator, false, false, 0);
        warning_box.pack_start(&warning_label, false, false, 4);

        let danger_box = GtkBox::new(Orientation::Horizontal, 4);
        let danger_indicator = GtkBox::new(Orientation::Horizontal, 0);
        danger_indicator.set_size_request(16, 16);
        danger_indicator.style_context().add_class("level-danger");
        danger_indicator
            .style_context()
            .add_class("level-indicator");
        let danger_label = Label::new(Some("Danger (Clipping)"));
        danger_box.pack_start(&danger_indicator, false, false, 0);
        danger_box.pack_start(&danger_label, false, false, 4);

        level_key_box.pack_start(&safe_box, false, false, 0);
        level_key_box.pack_start(&warning_box, false, false, 0);
        level_key_box.pack_start(&danger_box, false, false, 0);

        meter_box.pack_start(&meter_info_label, false, false, 0);
        meter_box.pack_start(&left_channel_box, false, false, 6);
        meter_box.pack_start(&right_channel_box, false, false, 6);
        meter_box.pack_start(&Separator::new(Orientation::Horizontal), false, false, 12);
        meter_box.pack_start(&level_key_box, false, false, 0);

        // ===== RECONNECT BUTTON SECTION =====
        let (button_frame, button_box) = create_section_box("Manual Connection");

        let reconnect_button = Button::with_label("Re-connect Monitor");
        reconnect_button.set_tooltip_text(Some("Manually reconnect audio monitoring to PipeWire"));

        let button_info_label = Label::new(Some(
            "If monitoring stops working, use this button to manually reconnect to PipeWire audio output.",
        ));
        button_info_label.set_line_wrap(true);
        button_info_label.set_halign(gtk::Align::Start);

        button_box.pack_start(&reconnect_button, false, false, 0);
        button_box.pack_start(&button_info_label, false, false, 0);

        // ===== ASSEMBLE TAB =====
        container.pack_start(&status_frame, false, false, 0);
        container.pack_start(&config_frame, false, false, 0);
        container.pack_start(&device_frame, false, false, 0);
        container.pack_start(&meter_frame, false, false, 0);
        container.pack_start(&button_frame, false, false, 0);

        // Create channel for thread communication
        let (sender, receiver) = mpsc::channel();

        let tab = MonitoringTab {
            container,
            status_label,
            config_info_label,
            device_info_label,
            sample_rate_label,
            bit_depth_label,
            buffer_size_label,
            left_channel_meter,
            right_channel_meter,
            system_info_label,
            reconnect_button,
            update_thread_running: Arc::new(Mutex::new(false)),
            sender,
        };

        // Set up button click handler
        let tab_for_button = tab.clone();
        tab.reconnect_button.connect_clicked(move |_| {
            tab_for_button.manual_reconnect();
        });

        // Set up receiver in the main thread
        let tab_clone = tab.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            // Check for messages from the monitoring thread
            while let Ok(message) = receiver.try_recv() {
                tab_clone.handle_message(message);
            }
            ControlFlow::Continue
        });

        // ====== CRITICAL: START MONITORING FIRST ======
        println!("INFO: Starting audio monitoring...");
        tab.start_monitoring();

        // ====== THEN SCHEDULE DELAYED AUTO-CONNECT ======
        let tab_clone2 = tab.clone();
        std::thread::spawn(move || {
            // Wait 5 seconds for monitor to be fully initialized
            std::thread::sleep(std::time::Duration::from_secs(5));

            println!("INFO: Starting delayed auto-connect...");

            match crate::audio_capture::auto_connect_monitor_delayed() {
                Ok(_) => {
                    println!("INFO: ✓ Delayed auto-connect successful!");
                    let _ = tab_clone2
                        .sender
                        .send(MonitorMessage::Status("✓ Monitor connected".to_string()));
                }
                Err(e) => {
                    println!("WARN: Delayed auto-connect failed: {}", e);
                    let _ = tab_clone2
                        .sender
                        .send(MonitorMessage::Status(format!("⚠ {}", e)));
                }
            }
        });

        tab
    }

    fn load_css() {
        // Try to load CSS from embedded data or file
        let css_data = if cfg!(debug_assertions) {
            // Development: try to load from file first, fall back to embedded
            match std::fs::read_to_string("data/styles.css") {
                Ok(css) => css,
                Err(_) => {
                    // Fallback to embedded CSS
                    include_str!("../data/styles.css").to_string()
                }
            }
        } else {
            // Production: use embedded CSS
            include_str!("../data/styles.css").to_string()
        };

        // Create CSS provider
        let provider = gtk::CssProvider::new();

        if let Err(e) = provider.load_from_data(css_data.as_bytes()) {
            eprintln!("Failed to load CSS: {}", e);
            return;
        }

        // Add provider to default screen
        if let Some(screen) = gtk::gdk::Screen::default() {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn handle_message(&self, message: MonitorMessage) {
        match message {
            MonitorMessage::Status(text) => {
                if text.starts_with("BUTTON_RESET") {
                    self.reconnect_button.set_sensitive(true);
                    self.reconnect_button.set_label("Re-connect Monitor");

                    // If it's an error, also show dialog
                    if text.starts_with("BUTTON_RESET_ERROR:") {
                        let error_msg = text.trim_start_matches("BUTTON_RESET_ERROR:");
                        let dialog = gtk::MessageDialog::new(
                            None::<&gtk::Window>,
                            gtk::DialogFlags::MODAL,
                            gtk::MessageType::Error,
                            gtk::ButtonsType::Close,
                            &format!("Failed to reconnect to PipeWire:\n\n{}", error_msg),
                        );
                        dialog.run();
                        // SAFETY: Destroying a dialog after it has been run is safe
                        unsafe {
                            dialog.destroy();
                        }
                    }
                } else {
                    self.status_label.set_text(&text);
                }
            }
            MonitorMessage::Config {
                sample_rate,
                bit_depth,
                buffer_size,
                audio_system,
            } => {
                self.config_info_label.set_text(&format!(
                    "Active Configuration: {}Hz, {}bit, {} samples",
                    sample_rate, bit_depth, buffer_size
                ));

                self.sample_rate_label.set_text(&format!(
                    "Sample Rate: {} Hz ({:.1} kHz)",
                    sample_rate,
                    sample_rate as f64 / 1000.0
                ));

                self.bit_depth_label
                    .set_text(&format!("Bit Depth: {} bit", bit_depth));

                self.buffer_size_label.set_text(&format!(
                    "Buffer Size: {} samples ({:.1}ms @ {}Hz)",
                    buffer_size,
                    (buffer_size as f64 * 1000.0) / sample_rate as f64,
                    sample_rate
                ));

                self.system_info_label
                    .set_text(&format!("Audio System: {}", audio_system));
            }
            MonitorMessage::Devices { output, input } => {
                self.device_info_label
                    .set_text(&format!("Output: {}\nInput: {}", output, input));
            }
            MonitorMessage::Levels {
                left_level,
                left_db,
                right_level,
                right_db,
            } => {
                // Update left channel meter
                self.left_channel_meter.set_fraction(left_level);
                self.left_channel_meter.set_text(Some(&left_db));

                // Apply CSS classes based on level - CORRECTED THRESHOLDS
                // NOTE: CSS has .level-safe, .level-warning, .level-danger classes
                let left_context = self.left_channel_meter.style_context();
                left_context.remove_class("level-safe");
                left_context.remove_class("level-warning");
                left_context.remove_class("level-danger");
                left_context.remove_class("clipping"); // This class is in CSS with animation

                // Correct thresholds that match your labels
                if left_level < 0.95 {
                    // Safe: < -3 dB
                    left_context.add_class("level-safe");
                } else if left_level < 0.99 {
                    // Warning: -3 dB to -0.6 dB
                    left_context.add_class("level-warning");
                } else {
                    // Danger: ≥ -0.6 dB (approaching clipping)
                    left_context.add_class("level-danger");
                    left_context.add_class("clipping"); // Add blinking effect for clipping
                }

                // Update right channel meter (same logic)
                self.right_channel_meter.set_fraction(right_level);
                self.right_channel_meter.set_text(Some(&right_db));

                let right_context = self.right_channel_meter.style_context();
                right_context.remove_class("level-safe");
                right_context.remove_class("level-warning");
                right_context.remove_class("level-danger");
                right_context.remove_class("clipping");

                if right_level < 0.95 {
                    right_context.add_class("level-safe");
                } else if right_level < 0.99 {
                    right_context.add_class("level-warning");
                } else {
                    right_context.add_class("level-danger");
                    right_context.add_class("clipping");
                }
            }

            MonitorMessage::Error(err) => {
                // Update status to show error
                self.status_label.set_text(&format!("Error: {}", err));
                eprintln!("Monitoring error: {}", err);

                // Also re-enable the button if it was disabled
                self.reconnect_button.set_sensitive(true);
                self.reconnect_button.set_label("Re-connect Monitor");
            }
        }
    }

    pub fn manual_reconnect(&self) {
        // Update UI directly (we're in main thread)
        self.status_label
            .set_text("Attempting manual reconnection...");
        self.reconnect_button.set_sensitive(false);
        self.reconnect_button.set_label("Connecting...");

        // Clone sender
        let sender = self.sender.clone();

        // Get the button's widget ID or use a flag approach
        thread::spawn(move || {
            // First, stop any existing monitoring
            println!("INFO: Stopping existing monitoring...");

            // Clear cache
            crate::audio::clear_cache();

            // Wait a moment
            thread::sleep(Duration::from_millis(500));

            // Try multiple connection methods
            let mut success = false;
            let mut error_message = String::new();

            // Method 1: Try the direct auto-connect
            println!("INFO: Attempting direct connection...");
            match crate::audio_capture::auto_connect_monitor_delayed() {
                Ok(_) => {
                    success = true;
                    println!("INFO: Direct connection successful");
                }
                Err(e) => {
                    error_message = format!("Direct connection failed: {}", e);
                    println!("WARN: {}", error_message);
                }
            }

            // Method 2: If direct connection fails, try manual pw-link commands
            if !success {
                println!("INFO: Trying manual pw-link connection...");
                match manual_pw_link_connection() {
                    Ok(_) => {
                        success = true;
                        println!("INFO: Manual pw-link connection successful");
                    }
                    Err(e) => {
                        error_message = format!("Manual connection also failed: {}", e);
                        println!("WARN: {}", error_message);
                    }
                }
            }

            if success {
                let _ = sender.send(MonitorMessage::Status(
                    "✓ Manual reconnection successful".to_string(),
                ));
                let _ = sender.send(MonitorMessage::Status("BUTTON_RESET".to_string()));
            } else {
                let _ = sender.send(MonitorMessage::Error(format!(
                    "Manual reconnection failed: {}",
                    error_message
                )));
                let _ = sender.send(MonitorMessage::Status(
                    "BUTTON_RESET_ERROR:".to_string() + &error_message,
                ));
            }
        });
    }

    pub fn start_monitoring(&self) {
        let sender = self.sender.clone();
        let running_clone = Arc::clone(&self.update_thread_running);

        // Store thread handles to avoid dropping them
        let _thread_handle = thread::spawn(move || {
            {
                let mut running = running_clone.lock().unwrap();
                *running = true;
            }

            let mut iteration = 0;

            // ====== CRITICAL: START THE MONITOR FIRST ======
            println!("INFO: Creating PipeWire monitor...");
            let monitor = PipeWireMonitor::new();

            // Update status with monitoring mode
            let monitoring_mode = monitor.get_monitoring_mode();
            let _ = sender.send(MonitorMessage::Status(format!("{} ●", monitoring_mode)));

            // Create a separate channel for audio levels
            let (audio_tx, audio_rx) = mpsc::channel();

            // ====== START THE MONITOR ======
            println!("INFO: Starting PipeWire audio monitoring...");
            let monitor_handle = match monitor.start(audio_tx) {
                Ok(handle) => {
                    println!("INFO: ✓ PipeWire monitor started successfully");
                    handle
                }
                Err(e) => {
                    let _ = sender.send(MonitorMessage::Error(format!(
                        "Failed to start audio monitor: {}",
                        e
                    )));
                    return;
                }
            };

            // Time-based rate limiting for expensive operations
            let mut last_config_check = Instant::now();
            let mut last_device_check = Instant::now();
            let config_check_interval = Duration::from_secs(2);
            let device_check_interval = Duration::from_secs(4);

            while {
                let running = running_clone.lock().unwrap();
                *running
            } {
                // Check for audio level updates
                if let Ok(levels) = audio_rx.try_recv() {
                    // Use peak levels for the meter display
                    let _ = sender.send(MonitorMessage::Levels {
                        left_level: levels.left_peak,
                        left_db: levels.left_db,
                        right_level: levels.right_peak,
                        right_db: levels.right_db,
                    });
                }

                // Update status indicator (blinking dot)
                let status_text = if iteration % 4 == 0 {
                    format!("{} ●", monitoring_mode)
                } else {
                    format!("{} ○", monitoring_mode)
                };

                let _ = sender.send(MonitorMessage::Status(status_text));

                // Update configuration information every 2 seconds
                if last_config_check.elapsed() >= config_check_interval {
                    match detect_current_audio_settings() {
                        Ok(settings) => {
                            let audio_system = detect_audio_system();

                            let _ = sender.send(MonitorMessage::Config {
                                sample_rate: settings.sample_rate,
                                bit_depth: settings.bit_depth,
                                buffer_size: settings.buffer_size,
                                audio_system,
                            });
                        }
                        Err(e) => {
                            let _ = sender.send(MonitorMessage::Error(format!(
                                "Config detection error: {}",
                                e
                            )));
                        }
                    }
                    last_config_check = Instant::now();
                }

                // Update device information every 4 seconds
                if last_device_check.elapsed() >= device_check_interval {
                    let output_device = detect_output_audio_device()
                        .unwrap_or_else(|e| format!("Error detecting output device: {}", e));

                    let input_device = detect_input_audio_device()
                        .unwrap_or_else(|e| format!("Error detecting input device: {}", e));

                    let _ = sender.send(MonitorMessage::Devices {
                        output: output_device,
                        input: input_device,
                    });
                    last_device_check = Instant::now();
                }

                iteration += 1;
                thread::sleep(Duration::from_millis(100));
            }

            // Stop the audio monitor when the main loop ends
            println!("INFO: Stopping PipeWire monitor...");
            monitor.stop();

            // Wait for monitor thread to finish
            let _ = monitor_handle.join();
            println!("INFO: PipeWire monitor stopped");
        });
    }

    pub fn stop_monitoring(&self) {
        let mut running = self.update_thread_running.lock().unwrap();
        *running = false;
    }

    pub fn refresh_now(&self) {
        // Clear cache to force fresh detection
        clear_audio_cache();

        // Force an immediate refresh by sending messages directly
        match detect_current_audio_settings() {
            Ok(settings) => {
                let audio_system = detect_audio_system();

                self.config_info_label.set_text(&format!(
                    "Active Configuration: {}Hz, {}bit, {} samples",
                    settings.sample_rate, settings.bit_depth, settings.buffer_size
                ));

                self.sample_rate_label.set_text(&format!(
                    "Sample Rate: {} Hz ({:.1} kHz)",
                    settings.sample_rate,
                    settings.sample_rate as f64 / 1000.0
                ));

                self.bit_depth_label
                    .set_text(&format!("Bit Depth: {} bit", settings.bit_depth));

                self.buffer_size_label.set_text(&format!(
                    "Buffer Size: {} samples ({:.1}ms @ {}Hz)",
                    settings.buffer_size,
                    (settings.buffer_size as f64 * 1000.0) / settings.sample_rate as f64,
                    settings.sample_rate
                ));

                self.system_info_label
                    .set_text(&format!("Audio System: {}", audio_system));
            }
            Err(e) => {
                self.config_info_label
                    .set_text(&format!("Error detecting configuration: {}", e));
            }
        }

        // Force update of device info
        glib::timeout_add_local(Duration::from_millis(10), {
            let device_info_label = self.device_info_label.clone();
            move || {
                let output_device = detect_output_audio_device()
                    .unwrap_or_else(|e| format!("Error detecting output device: {}", e));

                let input_device = detect_input_audio_device()
                    .unwrap_or_else(|e| format!("Error detecting input device: {}", e));

                device_info_label.set_text(&format!(
                    "Output: {}\nInput: {}",
                    output_device, input_device
                ));
                ControlFlow::Break
            }
        });
    }
}

fn create_section_box(title: &str) -> (Frame, GtkBox) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn init_gtk() -> bool {
        use std::sync::Once;
        static INIT: Once = Once::new();
        let mut success = true;
        INIT.call_once(|| {
            if gtk::init().is_err() {
                eprintln!("GTK initialization failed - skipping GUI tests");
                success = false;
            }
        });
        success
    }

    #[test]
    fn test_create_section_box() {
        // Skip if we can't initialize GTK (e.g., in CI without display)
        if !init_gtk() {
            println!("Skipping GUI test - no display available");
            return;
        }

        let (frame, section_box) = create_section_box("Test Section");
        assert!(frame.is_sensitive());
        assert_eq!(section_box.orientation(), Orientation::Vertical);
    }

    #[test]
    fn test_monitor_message_enum() {
        // This tests the enum, not audio functionality
        let status = MonitorMessage::Status("test".to_string());

        if let MonitorMessage::Status(text) = status {
            assert_eq!(text, "test");
        } else {
            panic!("Wrong variant");
        }
    }

    // Mark tests that need audio as #[ignore] for CI
    #[test]
    #[ignore = "Requires audio system - run locally only"]
    fn test_monitoring_tab_integration() {
        if !init_gtk() {
            return;
        }
        // ... integration test code
    }
}
