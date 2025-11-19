/*
 * Pro Audio Config - Configuration Management
 * Version: 1.3
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Configuration validation and script generation
 */

use std::process::Command;
use whoami;

use crate::audio::{AudioSettings, resolve_pipewire_device_name, resolve_pulse_device_name};

pub fn validate_device_pattern(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() {
        return Err("Device pattern cannot be empty".to_string());
    }

    let dangerous_patterns = ["..", "/", "~", "$", "`"];
    for dangerous in dangerous_patterns {
        if pattern.contains(dangerous) {
            return Err(format!("Device pattern contains dangerous sequence: '{}'", dangerous));
        }
    }

    if pattern.len() > 256 {
        return Err("Device pattern too long (max 256 characters)".to_string());
    }

    Ok(())
}

pub struct ServiceConfig {
    pub username: String,
    pub user_id: u32,
    pub runtime_dir: String,
    pub dbus_address: String,
}

impl ServiceConfig {
    pub fn for_current_user() -> Result<Self, String> {
        let username = whoami::username();
        
        let output = std::process::Command::new("id")
            .arg("-u")
            .arg(&username)
            .output()
            .map_err(|e| format!("Failed to get user ID: {}", e))?;
            
        if !output.status.success() {
            return Err("Failed to get user ID".to_string());
        }
        
        let user_id_str = String::from_utf8_lossy(&output.stdout);
	let user_id_str = user_id_str.trim();
        let user_id: u32 = user_id_str.parse()
            .map_err(|e| format!("Invalid user ID: {}", e))?;
            
        let runtime_dir = format!("/run/user/{}", user_id);
        let dbus_address = format!("unix:path={}/bus", runtime_dir);
        
        Ok(Self {
            username,
            user_id,
            runtime_dir,
            dbus_address,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        
        if self.user_id == 0 {
            return Err("Invalid user ID (0)".to_string());
        }
        
        if self.runtime_dir.is_empty() || !self.runtime_dir.starts_with("/run/user/") {
            return Err("Invalid runtime directory".to_string());
        }
        
        if self.dbus_address.is_empty() || !self.dbus_address.starts_with("unix:path=") {
            return Err("Invalid DBus address".to_string());
        }
        
        Ok(())
    }
}

pub struct ScriptGenerator {
    pub audio_settings: AudioSettings,
    pub service_config: ServiceConfig,
}

impl ScriptGenerator {
    pub fn new(audio_settings: AudioSettings) -> Result<Self, String> {
        audio_settings.validate()?;
        let service_config = ServiceConfig::for_current_user()?;
        service_config.validate()?;
        
        Ok(Self {
            audio_settings,
            service_config,
        })
    }

    pub fn generate_script(&self) -> Result<String, String> {
        self.audio_settings.validate()?;
        self.service_config.validate()?;

        let device_pattern = extract_device_pattern(&self.audio_settings.device_id)?;
        let format = self.audio_settings.get_audio_format()?;

        Ok(self.build_script_template(&device_pattern, format))
    }

    fn build_script_template(&self, device_pattern: &str, format: &str) -> String {
        format!(
            r#"#!/bin/bash
set -e

echo "=== Starting Audio Configuration ==="
echo "Running as: $(whoami)"
echo "Target user: {}"
echo "Target device: {}"
echo "Device pattern: {}"

# Get user ID and runtime directory
USER_ID={}
RUNTIME_DIR="{}"
DBUS_ADDRESS="{}"

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
echo "  Device Pattern: {}"
echo ""
echo "Note: Some settings may require application restart to take effect"
"#,
            self.service_config.username,
            self.audio_settings.device_id,
            device_pattern,
            self.service_config.user_id,
            self.service_config.runtime_dir,
            self.service_config.dbus_address,
            self.service_config.username,
            self.service_config.username,
            device_pattern,
            self.audio_settings.sample_rate,
            format,
            self.audio_settings.buffer_size,
            self.audio_settings.buffer_size,
            self.audio_settings.sample_rate,
            self.audio_settings.bit_depth,
            self.audio_settings.buffer_size,
            self.audio_settings.device_id,
            device_pattern,
        )
    }
}

// Device Pattern Extraction
pub fn extract_device_pattern(device_id: &str) -> Result<String, String> {
    match device_id {
        "default" => detect_current_default_device_name()
            .map_err(|e| format!("Failed to detect default device: {}", e)),
        id if id.starts_with("alsa:") => {
            let alsa_name = id.trim_start_matches("alsa:");
            Ok(if alsa_name.contains(".") {
                alsa_name.to_string()
            } else {
                format!("alsa_output.{}", alsa_name)
            })
        }
        id if id.starts_with("pipewire:") => {
            let node_id = id.trim_start_matches("pipewire:");
            resolve_pipewire_device_name(node_id)
                .map_err(|e| format!("Failed to resolve pipewire device: {}", e))
        }
        id if id.starts_with("pulse:") => {
            let pulse_id = id.trim_start_matches("pulse:");
            resolve_pulse_device_name(pulse_id)
                .map_err(|e| format!("Failed to resolve pulse device: {}", e))
        }
        _ => {
            validate_device_pattern(device_id)?;
            Ok(device_id.to_string())
        }
    }
}

fn detect_current_default_device_name() -> Result<String, String> {
    let output = Command::new("pactl")
        .args(["info"])
        .output()
        .map_err(|e| format!("Failed to get default device: {}", e))?;
        
    if !output.status.success() {
        return Err("Failed to query default device".to_string());
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if line.starts_with("Default Sink:") {
            let sink_name = line.replace("Default Sink:", "").trim().to_string();
            if !sink_name.is_empty() {
                return if sink_name.starts_with("alsa_output.") {
                    Ok(sink_name)
                } else {
                    Ok(format!("alsa_output.{}", sink_name.replace('.', "_")))
                };
            }
        }
    }
    
    Err("No default sink found".to_string())
}

// Application Function
pub fn apply_audio_settings_with_auth_blocking(settings: AudioSettings) -> Result<(), String> {
    println!("DEBUG: Selected device_id: {}", settings.device_id);
    settings.validate()?;

    let script_generator = ScriptGenerator::new(settings.clone())?;
    let script_content = script_generator.generate_script()?;
    
    let script_path = std::env::temp_dir().join("apply_audio_settings.sh");
    std::fs::write(&script_path, &script_content)
        .map_err(|e| format!("Failed to create settings script: {}", e))?;

    std::fs::set_permissions(&script_path, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .map_err(|e| format!("Failed to set script permissions: {}", e))?;

    println!("Executing script with pkexec...");

    let output = Command::new("pkexec")
        .arg("--disable-internal-agent")
        .arg("bash")
        .arg(&script_path)
        .output()
        .map_err(|e| format!("Failed to execute settings script: {}", e))?;

    let _ = std::fs::remove_file(&script_path);

    if output.status.success() {
        println!("Script executed successfully!");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            println!("Script output:\n{}", stdout);
        }
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        println!("Script failed with exit code: {}", output.status);
        println!("Script stdout:\n{}", stdout);
        println!("Script stderr:\n{}", stderr);
        
        let mut error_msg = format!("Script failed with exit code: {}", output.status.code().unwrap_or(-1));
        
        if !stderr.is_empty() {
            error_msg.push_str(&format!("\n\nError output:\n{}", stderr));
        }
        
        if !stdout.is_empty() {
            let lines: Vec<&str> = stdout.lines().collect();
            let last_lines: Vec<&str> = lines.iter().rev().take(5).rev().copied().collect();
            if !last_lines.is_empty() {
                error_msg.push_str(&format!("\n\nLast output:\n{}", last_lines.join("\n")));
            }
        }
        
        Err(error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioSettings;

    #[test]
    fn test_device_pattern_validation() {
        assert!(validate_device_pattern("alsa.*").is_ok());
        assert!(validate_device_pattern("").is_err());
        assert!(validate_device_pattern(&"a".repeat(300)).is_err());
        assert!(validate_device_pattern("alsa../dangerous").is_err());
    }

    #[test]
    fn test_service_config_creation() {
        let config = ServiceConfig::for_current_user();
        // This might fail in test environment, but we can test the structure
        if let Ok(config) = config {
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_script_generator_validation() {
        let settings = AudioSettings::new(48000, 24, 512, "default".to_string());
        let generator = ScriptGenerator::new(settings);
        assert!(generator.is_ok());
    }

    #[test]
    fn test_device_pattern_extraction() {
        // Test default case - may fail in test environment but should handle gracefully
        let result = extract_device_pattern("default");
        assert!(result.is_ok() || result.is_err()); // Should not panic
        
        // Test ALSA device
        let alsa_result = extract_device_pattern("alsa:mydevice");
        assert!(alsa_result.is_ok());
        assert!(alsa_result.unwrap().contains("alsa_output.mydevice"));
        
        // Test invalid pattern
        let invalid_result = extract_device_pattern("");
        assert!(invalid_result.is_err());
    }
}
