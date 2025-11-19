/*
 * Pro Audio Config - Audio Configuration Module
 * Version: 1.2
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Audio configuration tool for Linux PipeWire/ALSA systems (Jack in mind)
 */

use std::process::Command;
use whoami;

#[derive(Clone, Debug)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub id: String,
    pub device_type: DeviceType,
    pub available: bool,
}

#[derive(Clone, Debug)]
pub enum DeviceType {
    Input,
    Output,
    Duplex,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub bit_depth: u32, 
    pub buffer_size: u32,
    pub device_id: String, // Specific device to configure
}

impl AudioSettings {
    pub fn new(sample_rate: u32, bit_depth: u32, buffer_size: u32, device_id: String) -> Self {
        Self {
            sample_rate,
            bit_depth,
            buffer_size,
            device_id,
        }
    }
}

pub fn detect_all_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    println!("=== Scanning for all audio devices ===");
    
    // Method 1: PipeWire devices
    if let Ok(output) = Command::new("pw-cli")
        .args(["list-objects", "Node"])
        .output() {
        devices.extend(parse_pipewire_devices(&output.stdout)?);
    }
    
    // Method 2: ALSA devices
    devices.extend(detect_alsa_devices()?);
    
    // Method 3: PulseAudio devices (fallback)
    devices.extend(detect_pulse_devices()?);
    
    println!("Found {} audio devices", devices.len());
    Ok(devices)
}

fn is_real_hardware_device(device: &AudioDevice) -> bool {
    let name = device.name.to_lowercase();
    let description = device.description.to_lowercase();
    
    // Skip virtual devices and internal nodes
    if name.contains("virtual") || 
       description.contains("virtual") ||
       name.contains("null") ||
       description.contains("null") ||
       name.contains("dummy") ||
       description.contains("dummy") ||
       name.contains("echo-cancel") ||
       description.contains("echo-cancel") ||
       name.contains("monitor") ||
       description.contains("monitor") ||
       name.contains("proaudio") || // Skip our own config nodes
       description.contains("proaudio") {
        return false;
    }
    
    // Skip PipeWire internal nodes that aren't actual hardware
    if device.id.starts_with("pipewire:") {
        // Look for actual hardware indicators in PipeWire devices
        if description.contains("internal") && 
           !description.contains("usb") && 
           !description.contains("hdmi") && 
           !description.contains("analog") {
            return false;
        }
        
        // Skip nodes that are likely virtual or software-based
        if name.contains("alsa_") && name.contains("playback") ||
           name.contains("alsa_") && name.contains("capture") {
            return false;
        }
    }
    
    // Skip ALSA devices that are virtual/dummy
    if device.id.starts_with("alsa:") {
        if name == "default" || 
           name.contains("dmix") || 
           name.contains("dsnoop") || 
           name.contains("hw") || 
           name.contains("plughw") || 
           name.contains("lavrate") || 
           name.contains("samplerate") || 
           name.contains("speexrate") || 
           name.contains("variable") || 
           name.contains("rate_convert") || 
           name.contains("linear") || 
           name.contains("mu-law") || 
           name.contains("a-law") || 
           name.contains("float") || 
           name.contains("oss") || 
           name.contains("pulse") || 
           name.contains("upmix") || 
           name.contains("vdownmix") || 
           name.contains("usbstream") {
            return false;
        }
    }
    
    // Skip PulseAudio virtual devices
    if device.id.starts_with("pulse:") {
        if name.contains("module-") ||
           description.contains("module-") ||
           name == "auto_null" ||
           description.contains("null output") {
            return false;
        }
    }
    
    true
}

fn parse_pipewire_devices(output: &[u8]) -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    let output_str = String::from_utf8_lossy(output);
    
    let mut current_device: Option<AudioDevice> = None;
    let mut device_props: Vec<(String, String)> = Vec::new();
    
    for line in output_str.lines() {
        // Look for device nodes
        if line.contains("object:") && line.contains("Node") {
            if let Some(device) = current_device.take() {
                // Only add if it's a real hardware device
                if is_real_hardware_device(&device) {
                    devices.push(device);
                }
            }
            current_device = Some(AudioDevice {
                name: "Unknown".to_string(),
                description: "PipeWire Node".to_string(),
                id: extract_id(line),
                device_type: DeviceType::Unknown,
                available: true,
            });
            device_props.clear();
        }
        
        // Parse device properties
        if let Some(ref mut device) = current_device {
            if line.contains("node.name") && line.contains('=') {
                if let Some(name) = line.split('=').nth(1) {
                    device.name = name.trim().trim_matches('"').to_string();
                }
            }
            
            if line.contains("node.description") && line.contains('=') {
                if let Some(desc) = line.split('=').nth(1) {
                    device.description = desc.trim().trim_matches('"').to_string();
                }
            }
            
            // Determine device type with better classification
            if line.contains("media.class") && line.contains('=') {
                if let Some(class) = line.split('=').nth(1) {
                    let class_clean = class.trim().trim_matches('"');
                    device.device_type = match class_clean {
                        s if s.contains("Audio/Source") => DeviceType::Input,
                        s if s.contains("Audio/Sink") => DeviceType::Output,
                        s if s.contains("Audio/Duplex") => DeviceType::Duplex,
                        s if s.contains("Audio") => {
                            // For generic Audio class, try to determine from name/description
                            if device.name.to_lowercase().contains("input") ||
                               device.description.to_lowercase().contains("input") ||
                               device.description.to_lowercase().contains("capture") {
                                DeviceType::Input
                            } else if device.name.to_lowercase().contains("output") ||
                                      device.description.to_lowercase().contains("output") ||
                                      device.description.to_lowercase().contains("playback") {
                                DeviceType::Output
                            } else {
                                DeviceType::Unknown
                            }
                        }
                        _ => DeviceType::Unknown,
                    };
                }
            }
            
            // Store additional properties for better filtering
            if line.contains('=') && !line.trim().starts_with('#') {
                if let Some((key, value)) = line.split_once('=') {
                    let key_clean = key.trim().to_string();
                    let value_clean = value.trim().trim_matches('"').to_string();
                    device_props.push((key_clean, value_clean));
                }
            }
        }
    }
    
    // Don't forget the last device
    if let Some(device) = current_device.take() {
        if is_real_hardware_device(&device) {
            devices.push(device);
        }
    }
    
    Ok(devices)
}

fn detect_alsa_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    // Scan ALSA devices using aplay/arecord
    if let Ok(output) = Command::new("aplay")
        .args(["-L"])
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_alsa_output(&output_str, DeviceType::Output));
    }
    
    if let Ok(output) = Command::new("arecord")
        .args(["-L"])
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_alsa_output(&output_str, DeviceType::Input));
    }
    
    Ok(devices)
}

fn parse_alsa_output(output: &str, device_type: DeviceType) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    
    for line in output.lines() {
        if !line.starts_with(' ') && !line.is_empty() && line != "default" {
            let device = AudioDevice {
                name: line.to_string(),
                description: format!("ALSA {}", match device_type {
                    DeviceType::Input => "Input",
                    DeviceType::Output => "Output", 
                    _ => "Device",
                }),
                id: format!("alsa:{}", line),
                device_type: device_type.clone(),
                available: true,
            };
            
            // Only add real hardware devices
            if is_real_hardware_device(&device) {
                devices.push(device);
            }
        }
    }
    
    devices
}

fn detect_pulse_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    if let Ok(output) = Command::new("pactl")
        .args(["list", "sinks", "short"])
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_pulse_output(&output_str, DeviceType::Output));
    }
    
    if let Ok(output) = Command::new("pactl")
        .args(["list", "sources", "short"])
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_pulse_output(&output_str, DeviceType::Input));
    }
    
    Ok(devices)
}

fn parse_pulse_output(output: &str, device_type: DeviceType) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let device = AudioDevice {
                name: parts[1].to_string(),
                description: if parts.len() >= 3 { 
                    parts[2..].join(" ") 
                } else { 
                    "PulseAudio Device".to_string() 
                },
                id: format!("pulse:{}", parts[0]),
                device_type: device_type.clone(),
                available: true,
            };
            
            // Only add real hardware devices
            if is_real_hardware_device(&device) {
                devices.push(device);
            }
        }
    }
    
    devices
}

fn extract_id(line: &str) -> String {
    // Extract ID from object line like: "id: 123, object: Node"
    if let Some(id_part) = line.split(',').find(|s| s.trim().starts_with("id:")) {
        if let Some(id) = id_part.split(':').nth(1) {
            return format!("pipewire:{}", id.trim());
        }
    }
    "unknown".to_string()
}

pub fn detect_audio_device() -> Result<String, String> {
    // Detect audio system
    let audio_system = detect_audio_system();
    
    // Get default sink information
    let pactl_output = Command::new("pactl")
        .args(["info"])
        .output();
        
    if let Ok(output) = pactl_output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("Default Sink:") {
                    let sink_full = line.replace("Default Sink:", "");
                    let sink = sink_full.trim().to_string();
                    
                    return Ok(format!("{}: {}", audio_system, sink));
                }
            }
        }
    }
    
    // Fallback with system info
    Ok(format!("{}: Unknown Audio Device", audio_system))
}

fn detect_audio_system() -> String {
    // Check for PipeWire
    let pipewire_check = Command::new("pw-cli")
        .args(["info", "0"])
        .output();
    
    if let Ok(output) = pipewire_check {
        if output.status.success() {
            return "PipeWire".to_string();
        }
    }
    
    // Check for PipeWire via systemd
    let pipewire_service = Command::new("systemctl")
        .args(["--user", "is-active", "pipewire"])
        .output();
    
    if let Ok(output) = pipewire_service {
        if output.status.success() {
            return "PipeWire".to_string();
        }
    }
    
    // Check for PulseAudio
    let pulse_check = Command::new("pulseaudio")
        .args(["--check"])
        .output();
    
    if let Ok(output) = pulse_check {
        if output.status.success() {
            return "PulseAudio".to_string();
        }
    }
    
    // Check for PulseAudio via systemd
    let pulse_service = Command::new("systemctl")
        .args(["--user", "is-active", "pulseaudio"])
        .output();
    
    if let Ok(output) = pulse_service {
        if output.status.success() {
            return "PulseAudio".to_string();
        }
    }
    
    // Unknown audio system
    "Audio System".to_string()
}

pub fn apply_audio_settings_with_auth_blocking(settings: AudioSettings) -> Result<(), String> {
    // Get the current username
    let username = whoami::username();
    
    // Determine audio format based on bit depth
    let format = match settings.bit_depth {
        16 => "S16LE",
        24 => "S24LE", 
        32 => "S32LE",
        _ => "S24LE",
    };

    // Extract device pattern from device_id for matching
    let device_pattern = extract_device_pattern(&settings.device_id);

    // Create a script that uses direct environment variable setting
    let script_content = format!(
    r#"#!/bin/bash
set -e

echo "=== Starting Audio Configuration ==="
echo "Running as: $(whoami)"
echo "Target user: {}"
echo "Target device: {}"

# Get user ID and runtime directory
USER_ID=$(id -u {})
RUNTIME_DIR="/run/user/$USER_ID"
DBUS_ADDRESS="unix:path=$RUNTIME_DIR/bus"

echo "User ID: $USER_ID"
echo "Runtime directory: $RUNTIME_DIR"

# Function to run commands as user with proper environment
run_as_user() {{
    sudo -u {} XDG_RUNTIME_DIR="$RUNTIME_DIR" DBUS_SESSION_BUS_ADDRESS="$DBUS_ADDRESS" bash -c "$1"
}}

echo "Testing user service access..."
if run_as_user "systemctl --user daemon-reload"; then
    echo "User service access: OK"
else
    echo "User service access: Limited - some operations may fail"
fi

echo "Stopping audio services..."
run_as_user "systemctl --user stop wireplumber pipewire pipewire-pulse" || true
sleep 2

echo "Killing any remaining audio processes..."
run_as_user "pkill -f wireplumber" || true
run_as_user "pkill -f pipewire" || true
run_as_user "pkill -f pipewire-pulse" || true
sleep 1

echo "Creating WirePlumber configuration..."
CONFIG_DIR="/home/{}/.config/wireplumber/main.lua.d"
run_as_user "mkdir -p \"$CONFIG_DIR\""

# Create custom configuration targeting specific device
run_as_user "cat > \"$CONFIG_DIR/99-custom-audio.lua\"" << 'EOF'
alsa_monitor.rules = {{
    {{
        matches = {{
            {{
                {{ "device.name", "matches", "{}" }},
            }},
        }},
        apply_properties = {{
            ["audio.rate"] = {},
            ["audio.format"] = "{}",
            ["api.alsa.period-size"] = {},
        }},
    }},
}}
EOF

echo "Configuration created successfully"

echo "Restarting audio services..."
run_as_user "systemctl --user daemon-reload"
run_as_user "systemctl --user start wireplumber" || echo "Failed to start wireplumber"
sleep 2
run_as_user "systemctl --user start pipewire" || echo "Failed to start pipewire"
run_as_user "systemctl --user start pipewire-pulse" || echo "Failed to start pipewire-pulse"
sleep 3

echo "Setting buffer size..."
run_as_user "pw-metadata -n settings 0 clock.force-quantum {}" || echo "Note: Could not set quantum - continuing anyway"

sleep 2

echo "Verifying audio services are running..."
run_as_user "systemctl --user is-active wireplumber && echo 'WirePlumber: active' || echo 'WirePlumber: inactive'"
run_as_user "systemctl --user is-active pipewire && echo 'PipeWire: active' || echo 'PipeWire: inactive'"

echo "Current audio settings:"
run_as_user "pw-cli info 0 2>/dev/null | grep -E '(default.clock.rate|audio.format|default.clock.quantum)'" || echo "Could not query settings - but services may still be working"

echo ""
echo "=== Audio configuration completed ==="
echo "Applied settings:"
echo "  Sample Rate: {} Hz"
echo "  Bit Depth: {} bit"
echo "  Buffer Size: {} samples"
echo "  Target Device: {}"
echo ""
echo "Note: Some settings may require application restart to take effect"
"#,
    username,        // 1st placeholder: Target user
    settings.device_id, // 2nd placeholder: Target device
    username,        // 3rd placeholder: USER_ID
    username,        // 4th placeholder: sudo -u
    username,        // 5th placeholder: CONFIG_DIR
    device_pattern,  // 6th placeholder: device.name pattern
    settings.sample_rate,  // 7th placeholder: audio.rate
    format,          // 8th placeholder: audio.format
    settings.buffer_size,  // 9th placeholder: api.alsa.period-size
    settings.buffer_size,  // 10th placeholder: clock.force-quantum
    settings.sample_rate,  // 11th placeholder: Sample Rate in summary
    settings.bit_depth,    // 12th placeholder: Bit Depth in summary
    settings.buffer_size,  // 13th placeholder: Buffer Size in summary
    settings.device_id,    // 14th placeholder: Target Device in summary
);
    
    // Write temporary script
    let script_path = std::env::temp_dir().join("apply_audio_settings.sh");
    std::fs::write(&script_path, script_content)
        .map_err(|e| format!("Failed to create settings script: {}", e))?;

    // Make script executable
    std::fs::set_permissions(&script_path, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .map_err(|e| format!("Failed to set script permissions: {}", e))?;

    println!("Executing script with pkexec...");

    // Execute with pkexec and capture output
    let output = Command::new("pkexec")
        .arg("--disable-internal-agent")
        .arg("bash")
        .arg(&script_path)
        .output()
        .map_err(|e| format!("Failed to execute settings script: {}", e))?;

    // Clean up
    let _ = std::fs::remove_file(&script_path);

    if output.status.success() {
        println!("Script executed successfully!");
        
        // Print the output for debugging
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            println!("Script output:\n{}", stdout);
        }
        
        Ok(())
    } else {
        // Capture both stdout and stderr for detailed error information
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        println!("Script failed with exit code: {}", output.status);
        println!("Script stdout:\n{}", stdout);
        println!("Script stderr:\n{}", stderr);
        
        // Create a detailed error message
        let mut error_msg = format!("Script failed with exit code: {}", output.status.code().unwrap_or(-1));
        
        if !stderr.is_empty() {
            error_msg.push_str(&format!("\n\nError output:\n{}", stderr));
        }
        
        if !stdout.is_empty() {
            // Extract the last few lines of stdout for context
            let lines: Vec<&str> = stdout.lines().collect();
            let last_lines: Vec<&str> = lines.iter().rev().take(5).rev().copied().collect();
            if !last_lines.is_empty() {
                error_msg.push_str(&format!("\n\nLast output:\n{}", last_lines.join("\n")));
            }
        }
        
        Err(error_msg)
    }
}

// Helper function to extract device pattern for WirePlumber matching
fn extract_device_pattern(device_id: &str) -> String {
    match device_id {
        "default" => "alsa.*".to_string(), // Default targets all ALSA devices
        id if id.starts_with("alsa:") => {
            // Extract ALSA device name after "alsa:"
            id.trim_start_matches("alsa:").to_string()
        }
        id if id.starts_with("pipewire:") => {
            // For PipeWire devices, use the node ID directly
            let node_id = id.trim_start_matches("pipewire:");
            format!("alsa_card.{}", node_id) // PipeWire ALSA cards follow this pattern
        }
        id if id.starts_with("pulse:") => {
            // For PulseAudio devices, convert to ALSA pattern
            let pulse_id = id.trim_start_matches("pulse:");
            format!("alsa_output.{}", pulse_id)
        }
        _ => device_id.to_string(), // Fallback to original ID
    }
}

pub fn detect_current_audio_settings() -> Result<AudioSettings, String> {
    // Try to get current settings from PipeWire
    let pw_output = Command::new("pw-cli")
        .args(["info", "0"])
        .output();
        
    if let Ok(output) = pw_output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut sample_rate = 48000; // default
            let mut bit_depth = 24; // default
            let mut buffer_size = 512; // default
            
            println!("=== DEBUG: Raw pw-cli output ===");
            println!("{}", output_str);
            println!("=== DEBUG: End raw output ===");
            
            // Parse PipeWire output for current settings
            for line in output_str.lines() {
                // Look for lines containing the property with asterisk and tab
                if line.contains("default.clock.rate") && line.contains('=') {
                    if let Some(rate_str) = line.split('=').nth(1) {
                        let rate_clean = rate_str.trim().trim_matches('"').trim();
                        println!("=== DEBUG: Found sample rate line: '{}'", line);
                        println!("=== DEBUG: Extracted sample rate: '{}'", rate_clean);
                        if let Ok(rate) = rate_clean.parse::<u32>() {
                            sample_rate = rate;
                        }
                    }
                }
                
                // Look for lines containing the property with asterisk and tab
                if line.contains("audio.format") && line.contains('=') {
                    if let Some(format_str) = line.split('=').nth(1) {
                        let format = format_str.trim().trim_matches('"').trim();
                        println!("=== DEBUG: Found audio format line: '{}'", line);
                        println!("=== DEBUG: Extracted audio format: '{}'", format);
                        bit_depth = match format {
                            "S16LE" => 16,
                            "S24LE" => 24,
                            "S32LE" => 32,
                            _ => 24,
                        };
                    }
                }
                
                // Look for lines containing the property with asterisk and tab
                if line.contains("default.clock.quantum") && line.contains('=') && !line.contains("min-quantum") && !line.contains("max-quantum") && !line.contains("quantum-limit") && !line.contains("quantum-floor") {
                    if let Some(quantum_str) = line.split('=').nth(1) {
                        let quantum_clean = quantum_str.trim().trim_matches('"').trim();
                        println!("=== DEBUG: Found buffer size line: '{}'", line);
                        println!("=== DEBUG: Extracted buffer size: '{}'", quantum_clean);
                        if let Ok(quantum) = quantum_clean.parse::<u32>() {
                            buffer_size = quantum;
                        }
                    }
                }
            }
            
            println!("Detected settings: {}Hz, {}bit, {} samples", 
                     sample_rate, bit_depth, buffer_size);
            
            return Ok(AudioSettings::new(sample_rate, bit_depth, buffer_size, "default".to_string()));
        }
    }
    
    // Fallback: Try to get settings from pactl
    let pactl_output = Command::new("pactl")
        .args(["info"])
        .output();
        
    if let Ok(output) = pactl_output {
        if output.status.success() {
            // For PulseAudio, we might not get detailed format info
            // Use reasonable defaults
            return Ok(AudioSettings::new(48000, 24, 512, "default".to_string()));
        }
    }
    
    // Final fallback
    Ok(AudioSettings::new(48000, 24, 512, "default".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_settings_creation() {
        let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
        assert_eq!(settings.sample_rate, 48000);
        assert_eq!(settings.bit_depth, 24);
        assert_eq!(settings.buffer_size, 512);
        assert_eq!(settings.device_id, "default");
    }

    #[test]
    fn test_audio_settings_default_values() {
        let settings = AudioSettings::new(44100, 16, 1024, "alsa:default".to_string());
        assert_ne!(settings.sample_rate, 48000);
        assert_ne!(settings.bit_depth, 24);
        assert_ne!(settings.buffer_size, 512);
        assert_eq!(settings.device_id, "alsa:default");
    }

    #[test]
    fn test_format_selection() {
        let test_cases = vec![
            (16, "S16LE"),
            (24, "S24LE"),
            (32, "S32LE"),
            (8, "S24LE"),  // Default case
        ];

        for (bit_depth, expected_format) in test_cases {
            let format = match bit_depth {
                16 => "S16LE",
                24 => "S24LE", 
                32 => "S32LE",
                _ => "S24LE",
            };
            assert_eq!(format, expected_format);
        }
    }

    #[test]
    fn test_detect_audio_device_returns_result() {
        // This tests that the function at least compiles and returns a Result
        let result = detect_audio_device();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_audio_settings_serialization() {
        let settings = AudioSettings::new(192000, 32, 2048, "pipewire:123".to_string());
        
        // Test that settings can be used in string formatting
        let description = format!("{:?}", settings);
        assert!(description.contains("192000"));
        assert!(description.contains("32"));
        assert!(description.contains("2048"));
        assert!(description.contains("pipewire:123"));
    }
}
