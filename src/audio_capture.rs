/*
 * Pro Audio Config - Audio Capture Module
 * Version: 1.9
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 *
 * Real-time audio monitoring with auto-detection
 */

use bytemuck;
use std::f32;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::audio::{clear_cache, detect_output_audio_device, extract_actual_device_name};

// ====== AUTO-CONNECT FUNCTION (PUBLIC, MODULE LEVEL) ======

pub fn auto_connect_monitor_delayed() -> Result<(), String> {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    println!("DEBUG: Starting automatic audio connection...");
    thread::sleep(Duration::from_millis(1500));

    // 1. Get all monitor ports
    let output = Command::new("pw-link")
        .args(["--output"])
        .output()
        .map_err(|e| format!("Failed to run pw-link: {}", e))?;

    if !output.status.success() {
        return Err("pw-link command failed".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut monitor_ports = Vec::new();

    // 2. Parse all monitor ports, excluding our own app
    for line in output_str.lines() {
        if line.contains("monitor_") && !line.contains("pro_audio_config") {
            monitor_ports.push(line.trim().to_string());
        }
    }

    if monitor_ports.is_empty() {
        return Err(
            "No monitor ports found. Make sure audio is playing or an output device is active."
                .to_string(),
        );
    }

    println!("DEBUG: Found {} monitor ports:", monitor_ports.len());
    for port in &monitor_ports {
        println!("  - {}", port);
    }

    // 3. Group ports by their output device
    let mut devices: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for port in monitor_ports {
        // Extract device name (everything before the last colon)
        if let Some(colon_pos) = port.rfind(':') {
            let device = port[..colon_pos].to_string();
            let channel_port = port.to_string();
            devices
                .entry(device)
                .or_insert_with(Vec::new)
                .push(channel_port);
        }
    }

    // 4. Choose the best device (prefer active playback, fallback to first)
    let chosen_device = if devices.len() == 1 {
        // Only one device
        devices.keys().next().unwrap().clone()
    } else {
        // Multiple devices - try to detect which one is active
        // Check with pactl for active sink
        let active_device = if let Ok(pactl_output) = Command::new("pactl").args(["info"]).output()
        {
            let pactl_str = String::from_utf8_lossy(&pactl_output.stdout);
            pactl_str
                .lines()
                .find(|line| line.contains("Default Sink:"))
                .map(|line| line.replace("Default Sink:", "").trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        if !active_device.is_empty() {
            println!("DEBUG: Active audio sink: {}", active_device);
            // Find matching device
            devices
                .keys()
                .find(|device| device.contains(&active_device))
                .map(|d| d.clone())
                .unwrap_or_else(|| devices.keys().next().unwrap().clone())
        } else {
            // Fallback: choose device with most channels
            devices
                .iter()
                .max_by_key(|(_, ports)| ports.len())
                .map(|(device, _)| device.clone())
                .unwrap()
        }
    };

    let device_ports = &devices[&chosen_device];
    println!(
        "DEBUG: Selected device: {} ({} channels)",
        chosen_device,
        device_ports.len()
    );

    // 5. Connect each channel
    let mut successful_connections = 0;

    for monitor_port in device_ports {
        // Extract channel name (e.g., "FL" from "monitor_FL")
        let channel = if let Some(underscore_pos) = monitor_port.rfind('_') {
            &monitor_port[underscore_pos + 1..]
        } else {
            continue;
        };

        // Build target port name
        let target_port = format!("pro_audio_config:input_{}", channel);

        println!("DEBUG: Connecting {} -> {}", monitor_port, target_port);

        // Try to connect
        for attempt in 1..=3 {
            match Command::new("pw-link")
                .args([monitor_port, &target_port])
                .status()
            {
                Ok(status) if status.success() => {
                    println!("DEBUG: ✓ Connected {} channel", channel);
                    successful_connections += 1;
                    break;
                }
                Ok(_) if attempt < 3 => {
                    println!(
                        "DEBUG: Retrying {} channel (attempt {})...",
                        channel, attempt
                    );
                    thread::sleep(Duration::from_millis(300));
                }
                Err(e) if attempt < 3 => {
                    println!("DEBUG: Error connecting {}: {}, retrying...", channel, e);
                    thread::sleep(Duration::from_millis(300));
                }
                _ => {
                    println!("DEBUG: ✗ Failed to connect {} channel", channel);
                }
            }
        }
    }

    // 6. Verify and report
    thread::sleep(Duration::from_millis(500));

    // Check connections
    if let Ok(links_output) = Command::new("pw-link").args(["--links"]).output() {
        let links_str = String::from_utf8_lossy(&links_output.stdout);
        let our_links = links_str
            .lines()
            .filter(|line| line.contains("pro_audio_config"))
            .count();

        println!(
            "DEBUG: Active connections to pro_audio_config: {}",
            our_links
        );
    }

    if successful_connections > 0 {
        println!(
            "DEBUG: ✓ Successfully connected {}/{} channels",
            successful_connections,
            device_ports.len()
        );
        println!("DEBUG: Play audio to see levels in your app!");
        Ok(())
    } else {
        Err("Failed to connect any audio channels".to_string())
    }
}

// ====== AUDIO LEVELS STRUCT ======
#[derive(Debug, Clone)]
pub struct AudioLevels {
    pub left_peak: f64,
    pub right_peak: f64,
    pub left_db: String,
    pub right_db: String,
}

// ====== PIPE WIRE MONITOR ======
pub struct PipeWireMonitor {
    running: Arc<AtomicBool>,
    use_real_monitoring: Arc<AtomicBool>,
}

impl PipeWireMonitor {
    pub fn new() -> Self {
        // Auto-detect if PipeWire is available
        let use_real = Self::detect_pipewire_available();

        PipeWireMonitor {
            running: Arc::new(AtomicBool::new(false)),
            use_real_monitoring: Arc::new(AtomicBool::new(use_real)),
        }
    }

    /// Detect if PipeWire is available and running
    fn detect_pipewire_available() -> bool {
        // Method 1: Check if pw-dump command works
        if Command::new("pw-dump").arg("--version").output().is_ok() {
            return true;
        }

        // Method 2: Check if PipeWire service is running
        if let Ok(output) = Command::new("systemctl")
            .args(["--user", "is-active", "pipewire.service"])
            .output()
        {
            if output.status.success() {
                return true;
            }
        }

        // Method 3: Check if pipewire-pulse is running
        if let Ok(output) = Command::new("systemctl")
            .args(["--user", "is-active", "pipewire-pulse.service"])
            .output()
        {
            if output.status.success() {
                return true;
            }
        }

        // Method 4: Check for pipewire binary
        if Command::new("which").arg("pipewire").output().is_ok() {
            return true;
        }

        false
    }

    /// Get monitoring mode (real or simulation)
    pub fn get_monitoring_mode(&self) -> &'static str {
        if self.use_real_monitoring.load(Ordering::SeqCst) {
            "Real-time PipeWire Monitoring"
        } else {
            "Simulation Mode (PipeWire not detected)"
        }
    }

    pub fn start(
        &self,
        sender: mpsc::Sender<AudioLevels>,
    ) -> Result<thread::JoinHandle<()>, String> {
        let running = Arc::clone(&self.running);
        let use_real = self.use_real_monitoring.load(Ordering::SeqCst);

        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            if use_real {
                println!("INFO: Starting real PipeWire audio monitoring");

                // Clone sender for fallback if real monitoring fails
                let sender_clone = sender.clone();

                match Self::start_real_monitoring(Arc::clone(&running), sender) {
                    Ok(_) => {
                        println!("INFO: Real monitoring completed");
                        return;
                    }
                    Err(e) => {
                        eprintln!("WARNING: Real monitoring failed: {}", e);
                        eprintln!("Falling back to simulation mode");
                        // Use cloned sender for simulation
                        Self::start_simulation(Arc::clone(&running), sender_clone);
                    }
                }
            } else {
                // Start simulation directly
                println!("INFO: Starting simulated audio monitoring");
                Self::start_simulation(Arc::clone(&running), sender);
            }
        });

        Ok(handle)
    }

    fn start_simulation(running: Arc<AtomicBool>, sender: mpsc::Sender<AudioLevels>) {
        let mut iteration = 0;
        let start_time = Instant::now();

        while running.load(Ordering::SeqCst) {
            // Get audio levels (simulation)
            let levels = Self::simulate_audio_levels(iteration, start_time.elapsed().as_secs_f64());

            // Send levels to UI
            if sender.send(levels).is_err() {
                // Channel closed, stop monitoring
                break;
            }

            iteration += 1;
            thread::sleep(Duration::from_millis(100)); // 10 FPS update
        }
    }

    #[cfg(feature = "pipewire-monitoring")]
    fn start_real_monitoring(
        running: Arc<AtomicBool>,
        sender: mpsc::Sender<AudioLevels>,
    ) -> Result<(), String> {
        use libspa::pod::Pod;
        use libspa::utils::Direction;
        use pipewire as pw;
        use std::ffi::CString;
        use std::time::Duration;

        unsafe {
            pw::init();
        }

        // 1. Create main loop, context, and core
        let mainloop = pw::main_loop::MainLoopRc::new(None)
            .map_err(|e| format!("Failed to create MainLoop: {}", e))?;

        let context = pw::context::ContextRc::new(&mainloop, None)
            .map_err(|e| format!("Failed to create Context: {}", e))?;

        let core = context
            .connect_rc(None)
            .map_err(|e| format!("Failed to connect Core: {}", e))?;

        // 2. Create stream properties
        let props = pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::STREAM_CAPTURE_SINK => "true", // Enable monitor ports
        };

        // 3. Create the stream
        let stream = pw::stream::StreamBox::new(&core, "ProAudioMonitor", props)
            .map_err(|e| format!("Failed to create Stream: {}", e))?;

        println!("DEBUG: Stream created successfully");

        // 4. Start auto-connect in a separate thread
        let running_clone = Arc::clone(&running);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));

            if !running_clone.load(Ordering::SeqCst) {
                return;
            }

            println!("DEBUG: Auto-connecting to audio outputs...");
            // Call your existing auto_connect_monitor_delayed function
            if let Err(e) = auto_connect_monitor_delayed() {
                println!("WARN: Auto-connect failed: {}", e);
                println!("INFO: Manual connection may be needed");
            }
        });

        // 5. State for audio data
        struct AudioState {
            left_peak: f32,
            right_peak: f32,
            last_update: Option<std::time::Instant>,
        }

        // 6. Setup the stream listener
        let sender_clone = sender.clone();
        let _listener = stream
            .add_local_listener_with_user_data(AudioState {
                left_peak: 0.0,
                right_peak: 0.0,
                last_update: None,
            })
            .process(move |stream, user_data| {
                // Initialize last_update if needed
                if user_data.last_update.is_none() {
                    user_data.last_update = Some(std::time::Instant::now());
                }

                match stream.dequeue_buffer() {
                    None => {
                        // No buffer available
                    }
                    Some(mut buffer) => {
                        let datas = buffer.datas_mut();
                        if datas.is_empty() {
                            return;
                        }

                        // Process audio data
                        let data = &mut datas[0];
                        let chunk_size = data.chunk().size() as usize;

                        if let Some(samples) = data.data() {
                            let f32_slice: &[f32] = bytemuck::cast_slice(&samples[..chunk_size]);

                            // Find peak values (assuming stereo interleaved format)
                            for chunk in f32_slice.chunks(2) {
                                if let Some(&left) = chunk.get(0) {
                                    user_data.left_peak = user_data.left_peak.max(left.abs());
                                }
                                if let Some(&right) = chunk.get(1) {
                                    user_data.right_peak = user_data.right_peak.max(right.abs());
                                }
                            }
                        }

                        // Send updates periodically (every 100ms)
                        if let Some(last) = user_data.last_update {
                            if last.elapsed() >= Duration::from_millis(100) {
                                let left_db = 20.0 * (user_data.left_peak.max(0.0001).log10());
                                let right_db = 20.0 * (user_data.right_peak.max(0.0001).log10());

                                let left_level = ((left_db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;
                                let right_level = ((right_db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;

                                let _ = sender_clone.send(AudioLevels {
                                    left_peak: left_level,
                                    right_peak: right_level,
                                    left_db: format!("{:.1} dB", left_db),
                                    right_db: format!("{:.1} dB", right_db),
                                });

                                user_data.left_peak = 0.0;
                                user_data.right_peak = 0.0;
                                user_data.last_update = Some(std::time::Instant::now());
                            }
                        }
                    }
                }
            })
            .register()
            .map_err(|e| format!("Failed to register listener: {}", e))?;

        // 7. Set audio format
        let mut audio_info = libspa::param::audio::AudioInfoRaw::new();
        audio_info.set_format(libspa::param::audio::AudioFormat::F32LE);
        let obj = pw::spa::pod::Object {
            type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
            id: pw::spa::param::ParamType::EnumFormat.as_raw(),
            properties: audio_info.into(),
        };
        let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pw::spa::pod::Value::Object(obj),
        )
        .unwrap()
        .0
        .into_inner();
        let mut params = [Pod::from_bytes(&values).unwrap()];

        // 8. Connect the stream
        stream
            .connect(
                Direction::Input,
                None,
                pw::stream::StreamFlags::AUTOCONNECT
                    | pw::stream::StreamFlags::MAP_BUFFERS
                    | pw::stream::StreamFlags::RT_PROCESS,
                &mut params,
            )
            .map_err(|e| format!("Failed to connect stream: {}", e))?;

        println!("DEBUG: Stream connected with AUTOCONNECT flag");

        // 9. Run the main loop
        while running.load(Ordering::SeqCst) {
            let timeout = Duration::from_millis(10);
            mainloop.loop_().iterate(timeout);

            // Small sleep to prevent CPU spin
            thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }

    #[cfg(not(feature = "pipewire-monitoring"))]
    fn start_real_monitoring(
        _running: Arc<AtomicBool>,
        _sender: mpsc::Sender<AudioLevels>,
    ) -> Result<(), String> {
        Err(
            "PipeWire feature not compiled in. Rebuild with --features pipewire-monitoring"
                .to_string(),
        )
    }

    fn simulate_audio_levels(iteration: i32, elapsed_time: f64) -> AudioLevels {
        // Simulate some audio activity with more realistic patterns
        let base_time = elapsed_time;

        // Left channel - simulate various audio patterns
        let left_value = 0.3 + 0.4 * (base_time * 0.8).sin().abs();
        let left_peak = if iteration % 23 == 0 {
            // Occasional peaks
            0.85 + 0.15 * (base_time * 12.0).sin().abs()
        } else if iteration % 7 == 0 {
            // Smaller peaks
            0.6 + 0.3 * (base_time * 5.0).cos().abs()
        } else {
            0.0
        };

        let left_level = left_value.max(left_peak).min(1.0);
        let left_db = 20.0 * (left_level + 0.0001).log10();

        // Right channel - similar but slightly different pattern
        let right_value = 0.25 + 0.35 * (base_time * 0.7).sin().abs();
        let right_peak = if iteration % 31 == 0 {
            // Occasional peaks
            0.9 + 0.1 * (base_time * 15.0).cos().abs()
        } else if iteration % 11 == 0 {
            // Smaller peaks
            0.55 + 0.35 * (base_time * 3.0).sin().abs()
        } else {
            0.0
        };

        let right_level = right_value.max(right_peak).min(1.0);
        let right_db = 20.0 * (right_level + 0.0001).log10();

        AudioLevels {
            left_peak: left_level,
            right_peak: right_level,
            left_db: format!("{:.1} dB", left_db),
            right_db: format!("{:.1} dB", right_db),
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_levels_struct() {
        let levels = AudioLevels {
            left_peak: 0.5,
            right_peak: 0.6,
            left_db: "-6.0 dB".to_string(),
            right_db: "-4.0 dB".to_string(),
        };

        assert_eq!(levels.left_peak, 0.5);
        assert_eq!(levels.right_peak, 0.6);
        assert!(levels.left_db.contains("dB"));
        assert!(levels.right_db.contains("dB"));
    }

    #[test]
    fn test_simulation() {
        let levels = PipeWireMonitor::simulate_audio_levels(0, 0.0);

        assert!(levels.left_peak >= 0.0 && levels.left_peak <= 1.0);
        assert!(levels.right_peak >= 0.0 && levels.right_peak <= 1.0);
        assert!(levels.left_db.contains("dB"));
        assert!(levels.right_db.contains("dB"));
    }

    #[test]
    fn test_monitor_creation() {
        let monitor = PipeWireMonitor::new();
        // Should create without panicking
        assert!(true);
    }

    #[test]
    fn test_pipewire_detection_logic() {
        // Test that the detection logic doesn't panic
        let _ = PipeWireMonitor::detect_pipewire_available();
        assert!(true);
    }
}
