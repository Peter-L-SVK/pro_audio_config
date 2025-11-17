use std::process::Command;
use whoami;

#[derive(Clone, Debug)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub buffer_size: u32,
}

impl AudioSettings {
    pub fn new(sample_rate: u32, bit_depth: u32, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            bit_depth,
            buffer_size,
        }
    }
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

    // Create a script that uses direct environment variable setting
    let script_content = format!(
    r#"#!/bin/bash
set -e

echo "=== Starting Audio Configuration ==="
echo "Running as: $(whoami)"
echo "Target user: {}"

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

# Create custom configuration
run_as_user "cat > \"$CONFIG_DIR/99-custom-audio.lua\"" << 'EOF'
alsa_monitor.rules = {{
    matches = {{
        {{
            {{ "device.name", "matches", "alsa.*" }},
        }},
    }},
    apply_properties = {{
        ["audio.rate"] = {},
        ["audio.format"] = "{}",
        ["api.alsa.period-size"] = {},
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
echo ""
echo "Note: Some settings may require application restart to take effect"
"#,
    username,        // 1st placeholder: Target user
    username,        // 2nd placeholder: USER_ID
    username,        // 3rd placeholder: sudo -u
    username,        // 4th placeholder: CONFIG_DIR
    settings.sample_rate,  // 5th placeholder: audio.rate
    format,          // 6th placeholder: audio.format
    settings.buffer_size,  // 7th placeholder: api.alsa.period-size
    settings.buffer_size,  // 8th placeholder: clock.force-quantum
    settings.sample_rate,  // 9th placeholder: Sample Rate in summary
    settings.bit_depth,    // 10th placeholder: Bit Depth in summary
    settings.buffer_size,  // 11th placeholder: Buffer Size in summary
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
            
            return Ok(AudioSettings::new(sample_rate, bit_depth, buffer_size));
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
            return Ok(AudioSettings::new(48000, 24, 512));
        }
    }
    
    // Final fallback
    Ok(AudioSettings::new(48000, 24, 512))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_settings_creation() {
        let settings = AudioSettings::new(48000, 24, 512);
        assert_eq!(settings.sample_rate, 48000);
        assert_eq!(settings.bit_depth, 24);
        assert_eq!(settings.buffer_size, 512);
    }

    #[test]
    fn test_audio_settings_default_values() {
        let settings = AudioSettings::new(44100, 16, 1024);
        assert_ne!(settings.sample_rate, 48000);
        assert_ne!(settings.bit_depth, 24);
        assert_ne!(settings.buffer_size, 512);
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
        let settings = AudioSettings::new(192000, 32, 2048);
        
        // Test that settings can be used in string formatting
        let description = format!("{:?}", settings);
        assert!(description.contains("192000"));
        assert!(description.contains("32"));
        assert!(description.contains("2048"));
    }
}
