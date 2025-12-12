/*
 * Pro Audio Config - Audio Capture Module
 * Version: 1.8
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 *
 * Real-time audio monitoring with auto-detection
 */

use bytemuck;
use lazy_static::lazy_static;
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

    println!("DEBUG: Delayed auto-connect (waiting for full initialization)...");

    // Wait for everything to be fully initialized
    thread::sleep(Duration::from_millis(2000));

    // Clear cache to get fresh detection
    clear_cache();

    // 1. Use your EXISTING working detection
    let device_info = detect_output_audio_device()?;
    println!("DEBUG: Detected after delay: {}", device_info);

    // 2. Extract the device name
    let device_name = extract_actual_device_name(&device_info)
        .ok_or_else(|| format!("Could not extract device name from: {}", device_info))?;

    println!("DEBUG: Connecting to: {}", device_name);

    // 3. Get the EXACT ProAudioMonitor port from pw-dump
    println!("DEBUG: Getting ProAudioMonitor port details...");

    let monitor_port = if let Ok(output) = Command::new("pw-dump").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Look for "object.path": "ProAudioMonitor:input_0"
        if let Some(start) = output_str.find("\"object.path\": \"ProAudioMonitor:") {
            let after_start = &output_str[start + 16..]; // Skip "\"object.path\": \""
            if let Some(end) = after_start.find('"') {
                let port = &after_start[..end];
                println!("DEBUG: Found monitor port: {}", port);
                Some(port.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let monitor_port =
        monitor_port.ok_or_else(|| "Could not find ProAudioMonitor port".to_string())?;

    // 4. Get the EXACT output port from pw-link --output
    println!("DEBUG: Getting exact output port for {}...", device_name);

    let output_ports_output = Command::new("pw-link")
        .args(["--output"])
        .output()
        .map_err(|e| format!("Failed to check ports: {}", e))?;

    let mut target_port = None;

    if output_ports_output.status.success() {
        let ports_str = String::from_utf8_lossy(&output_ports_output.stdout);
        println!("DEBUG: Available output ports for {}:", device_name);

        for line in ports_str.lines() {
            if line.contains(&device_name) && line.contains("monitor_FL") {
                println!("  ✓ Found: {}", line);
                target_port = Some(line.to_string());
                break;
            }
        }
    }

    let target_port =
        target_port.ok_or_else(|| format!("Could not find monitor_FL port for {}", device_name))?;

    // 5. Connect using the EXACT port-to-port connection
    println!("DEBUG: Connecting {} to {}", target_port, monitor_port);

    let cmd = format!("{} {}", target_port, monitor_port);
    println!("DEBUG: Executing: {}", cmd);

    // Try multiple times
    for attempt in 1..=5 {
        println!("DEBUG: Connection attempt {}...", attempt);

        match Command::new("pw-link")
            .args(cmd.split_whitespace())
            .status()
        {
            Ok(status) if status.success() => {
                println!("DEBUG: ✓ Connection successful!");

                // Verify connection
                thread::sleep(Duration::from_millis(500));

                // Check with pw-cli inspect
                if let Ok(output) = Command::new("pw-cli")
                    .args(["inspect", "ProAudioMonitor"])
                    .output()
                {
                    if output.status.success() {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        if output_str.contains("links") && !output_str.contains("links = 0") {
                            println!("DEBUG: ✓ Connection verified in PipeWire!");
                        }
                    }
                }

                // Check with pw-top style monitoring
                println!("DEBUG: Connection should be active. Check audio levels in your app!");
                return Ok(());
            }
            Ok(_) => {
                println!("DEBUG: Connection failed, retrying...");
            }
            Err(e) => {
                println!("DEBUG: Error: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(1000));
    }

    // 6. Provide EXACT manual command
    println!("DEBUG: === MANUAL CONNECTION COMMAND ===");
    println!("DEBUG: Run this exact command in terminal:");
    println!("DEBUG:   pw-link {} {}", target_port, monitor_port);
    println!("DEBUG:");
    println!("DEBUG: Or if that doesn't work, try:");
    println!("DEBUG:   pw-link {} ProAudioMonitor:input_0", target_port);

    Err(format!(
        "Failed to connect {} to {}",
        target_port, monitor_port
    ))
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
        use std::process::Command;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Initialize PipeWire
        pw::init();

        // 1. Create main loop
        let mainloop = pw::main_loop::MainLoop::new(None)
            .map_err(|e| format!("Failed to create MainLoop: {}", e))?;

        let context = pw::context::Context::new(&mainloop)
            .map_err(|e| format!("Failed to create Context: {}", e))?;

        let core = context
            .connect(None)
            .map_err(|e| format!("Failed to connect Core: {}", e))?;

        // 2. Create properties - specifically for monitoring
        let mut props = pw::properties::Properties::new();

        props.insert(
            CString::new("media.type").unwrap().into_bytes(),
            CString::new("Audio").unwrap().into_bytes(),
        );
        props.insert(
            CString::new("media.category").unwrap().into_bytes(),
            CString::new("Capture").unwrap().into_bytes(),
        );
        props.insert(
            CString::new("node.name").unwrap().into_bytes(),
            CString::new("ProAudioMonitor").unwrap().into_bytes(),
        );
        props.insert(
            CString::new("audio.channels").unwrap().into_bytes(),
            CString::new("2").unwrap().into_bytes(),
        );
        props.insert(
            CString::new("node.description").unwrap().into_bytes(),
            CString::new("Audio Monitor for Pro Audio Config")
                .unwrap()
                .into_bytes(),
        );
        // Add monitor properties for better auto-connect
        props.insert(
            CString::new("stream.capture.sink").unwrap().into_bytes(),
            CString::new("true").unwrap().into_bytes(),
        );

        // 3. Create stream
        let stream = pw::stream::Stream::new(&core, "audio-monitor", props)
            .map_err(|e| format!("Failed to create Stream: {}", e))?;

        println!("DEBUG: Stream created successfully");

        // 4. Create state
        #[derive(Default)]
        struct AudioState {
            left_peak: f32,
            right_peak: f32,
            last_update: Option<std::time::Instant>,
        }

        let sender_clone = sender.clone();

        let _listener = stream
            .add_local_listener::<AudioState>()
            .process(
                move |stream_ref: &pw::stream::StreamRef, state: &mut AudioState| {
                    // Initialize last_update if needed
                    if state.last_update.is_none() {
                        state.last_update = Some(std::time::Instant::now());
                    }

                    if let Some(mut buffer) = stream_ref.dequeue_buffer() {
                        let mut has_audio = false;

                        for data in buffer.datas_mut() {
                            let chunk_ref = data.chunk();
                            let data_size_bytes = chunk_ref.size() as usize;

                            if let Some(data_slice) = data.data() {
                                if data_size_bytes > 0 {
                                    has_audio = true;

                                    let samples: &[f32] =
                                        bytemuck::cast_slice(&data_slice[..data_size_bytes]);

                                    for i in (0..samples.len()).step_by(2) {
                                        if i < samples.len() {
                                            state.left_peak = state.left_peak.max(samples[i].abs());
                                        }
                                        if i + 1 < samples.len() {
                                            state.right_peak =
                                                state.right_peak.max(samples[i + 1].abs());
                                        }
                                    }
                                }
                            }
                        }

                        // Send updates periodically
                        if let Some(last) = state.last_update {
                            if last.elapsed() >= Duration::from_millis(100) {
                                let left_db = 20.0 * (state.left_peak.max(0.0001).log10());
                                let right_db = 20.0 * (state.right_peak.max(0.0001).log10());

                                let left_level = ((left_db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;
                                let right_level = ((right_db + 60.0) / 60.0).clamp(0.0, 1.0) as f64;

                                let _ = sender_clone.send(AudioLevels {
                                    left_peak: left_level,
                                    right_peak: right_level,
                                    left_db: format!("{:.1} dB", left_db),
                                    right_db: format!("{:.1} dB", right_db),
                                });

                                state.left_peak = 0.0;
                                state.right_peak = 0.0;
                                state.last_update = Some(std::time::Instant::now());
                            }
                        }
                    } else {
                        // No buffer available
                        static NO_BUFFER_COUNT: AtomicUsize = AtomicUsize::new(0);
                        let count = NO_BUFFER_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                        if count == 1 || count % 50 == 0 {
                            println!(
                                "DEBUG: No audio buffer (count: {}) - monitor may not be connected",
                                count
                            );
                        }
                    }
                },
            )
            .register();

        // 5. Try PipeWire's auto-connect first
        let direction = Direction::Input;
        let mut params: Vec<&Pod> = Vec::new();

        // First, try PipeWire's auto-connect
        match stream.connect(
            direction,
            None,
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        ) {
            Ok(_) => println!("DEBUG: Stream connected with AUTOCONNECT flag"),
            Err(e) => println!("WARN: Stream auto-connect failed: {}", e),
        }

        // 6. Wait for stream to initialize
        std::thread::sleep(std::time::Duration::from_millis(1000));

        // 7. Run the main loop
        let loop_ref = mainloop.loop_();
        while running.load(Ordering::SeqCst) {
            loop_ref.iterate(Duration::from_millis(10));
            thread::sleep(Duration::from_millis(10));
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
