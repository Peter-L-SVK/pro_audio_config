/*
 * Pro Audio Config - Configuration Module  
 * Version: 1.6
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * 
 * Handles audio configuration for PipeWire and WirePlumber with authentication
 * and multiple fallback approaches.
 */

use std::path::Path;
use std::process::Command;
use std::fs;
use crate::audio::AudioSettings;

/// Applies output audio settings with authentication
pub fn apply_output_audio_settings_with_auth_blocking(settings: AudioSettings) -> Result<(), String> {
    apply_audio_settings_with_auth(settings, "output")
}

/// Applies input audio settings with authentication  
pub fn apply_input_audio_settings_with_auth_blocking(settings: AudioSettings) -> Result<(), String> {
    apply_audio_settings_with_auth(settings, "input")
}

/// Applies audio settings for user-specific configuration
pub fn apply_user_audio_settings(settings: AudioSettings, tab_type: &str) -> Result<(), String> {
    println!("Applying user-specific {} audio settings", tab_type);
    update_audio_settings(&settings, false) // false = not system-wide
}

/// Main function to apply audio settings with authentication
fn apply_audio_settings_with_auth(settings: AudioSettings, stream_type: &str) -> Result<(), String> {
    println!("Applying {} audio settings with authentication: {}Hz/{}bit/{} samples", 
             stream_type,
             settings.sample_rate, settings.bit_depth, settings.buffer_size);
    
    // First try the new multi-approach configuration
    match update_audio_settings(&settings, true) {
        Ok(()) => {
            println!("✓ Successfully applied {} settings using new configuration system", stream_type);
            return Ok(());
        }
        Err(e) => {
            println!("New configuration system failed: {}, falling back to legacy method...", e);
        }
    }

    // Fall back to legacy WirePlumber configuration (only for older versions)
    if should_use_legacy_wireplumber_config()? {
        apply_legacy_wireplumber_config(&settings, stream_type)
    } else {
        Err("Legacy WirePlumber configuration not applicable for current WirePlumber version".to_string())
    }
}

/// Execute a command with privilege escalation for system-wide changes
fn execute_with_privileges(command: &str, args: &[&str]) -> Result<(), String> {
    println!("Requesting administrator privileges for system-wide changes...");
    
    // Try pkexec first (common on modern Linux systems)
    let status = Command::new("pkexec")
        .arg(command)
        .args(args)
        .status()
        .map_err(|e| format!("Failed to execute with pkexec: {}. Make sure pkexec is available.", e))?;
    
    if status.success() {
        Ok(())
    } else {
        // Fallback to sudo
        println!("pkexec failed, trying sudo...");
        let status = Command::new("sudo")
            .arg(command)
            .args(args)
            .status()
            .map_err(|e| format!("Failed to execute with sudo: {}. Please run with administrator privileges.", e))?;
            
        if status.success() {
            Ok(())
        } else {
            Err("Privilege escalation failed. Please run as root or configure sudo/pkexec.".to_string())
        }
    }
}

/// Write configuration file with proper privilege escalation for system paths
fn write_config_with_privileges(config_path: &str, content: &str) -> Result<(), String> {
    if config_path.starts_with("/etc/") {
        // System path - need privileges
        let temp_file = format!("/tmp/pro-audio-config-{}", std::process::id());
        
        // Write to temp file first
        fs::write(&temp_file, content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        // Copy to system location with privileges
        execute_with_privileges("cp", &[&temp_file, config_path])?;
        
        // Clean up temp file
        let _ = fs::remove_file(&temp_file);
        
        println!("✓ System config created with privileges: {}", config_path);
    } else {
        // User path - no privileges needed
        if let Some(parent) = Path::new(config_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        
        fs::write(config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }
    
    Ok(())
}

/// Create directory with proper privilege escalation for system paths
fn create_dir_all_with_privileges(path: &str) -> Result<(), String> {
    if path.starts_with("/etc/") {
        // System path - need privileges
        execute_with_privileges("mkdir", &["-p", path])?;
    } else {
        // User path - no privileges needed
        fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    
    Ok(())
}

/// Checks if we should use legacy WirePlumber config (for versions < 0.5)
fn should_use_legacy_wireplumber_config() -> Result<bool, String> {
    match get_wireplumber_version() {
        Ok(version) => {
            println!("Detected WirePlumber version: {}", version);
            // Use legacy config only for versions older than 0.5
            let use_legacy = version < "0.5".to_string();
            if use_legacy {
                println!("Using legacy Lua configuration for WirePlumber < 0.5");
            } else {
                println!("Using modern JSON configuration for WirePlumber >= 0.5");
            }
            Ok(use_legacy)
        }
        Err(e) => {
            println!("Could not detect WirePlumber version: {}, assuming modern version", e);
            // If we can't detect version, assume modern and don't use legacy config
            Ok(false)
        }
    }
}

/// Gets the WirePlumber version as a string
fn get_wireplumber_version() -> Result<String, String> {
    // Method 1: Try wireplumber --version directly
    if let Ok(output) = Command::new("wireplumber")
        .args(["--version"])
        .output() {
        let version_output = String::from_utf8_lossy(&output.stdout);
        for line in version_output.lines() {
            if line.contains("WirePlumber") || line.contains("wireplumber") || line.contains("version") {
                // Look for version patterns like "0.5.12", "v0.4.14", etc.
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    // Extract version numbers like 0.5.12, 0.4.14, etc.
                    let version_str = part.trim_matches(|c: char| !c.is_numeric() && c != '.');
                    if version_str.chars().any(|c| c.is_numeric()) && version_str.contains('.') {
                        let version_parts: Vec<&str> = version_str.split('.').collect();
                        if version_parts.len() >= 2 {
                            return Ok(format!("{}.{}", version_parts[0], version_parts[1]));
                        }
                        return Ok(version_str.to_string());
                    }
                }
            }
        }
    }

    // Method 2: Try package managers (distribution-specific)
    let package_commands = [
        ("rpm", &["-q", "wireplumber"] as &[&str]),
        ("dpkg", &["-l", "wireplumber"]),
        ("pacman", &["-Q", "wireplumber"]),
        ("apk", &["info", "wireplumber"]),
    ];

    for (cmd, args) in &package_commands {
        if let Ok(output) = Command::new(cmd).args(*args).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Extract version from package manager output
                for line in output_str.lines() {
                    // Look for version patterns in package output
                    let words: Vec<&str> = line.split_whitespace().collect();
                    for word in words {
                        if word.chars().next().map_or(false, |c| c.is_numeric()) && word.contains('.') {
                            let version_parts: Vec<&str> = word.split('.').collect();
                            if version_parts.len() >= 2 {
                                return Ok(format!("{}.{}", version_parts[0], version_parts[1]));
                            }
                        }
                    }
                }
            }
        }
    }

    // Method 3: Try to find wireplumber binary and get version from path or ldd
    if let Ok(output) = Command::new("which").arg("wireplumber").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("Found wireplumber at: {}", path);
            
            // If we found the binary but can't get version, assume modern
            return Ok("0.5".to_string());
        }
    }

    // Method 4: Check common installation paths
    let common_paths = [
        "/usr/bin/wireplumber",
        "/usr/local/bin/wireplumber",
        "/bin/wireplumber",
    ];

    for path in &common_paths {
        if Path::new(path).exists() {
            println!("Found wireplumber at: {}", path);
            // If it exists but we can't determine version, assume modern
            return Ok("0.5".to_string());
        }
    }

    Err("Could not detect WirePlumber version - WirePlumber may not be installed".to_string())
}

/// Legacy WirePlumber configuration method (fallback for versions < 0.5)
fn apply_legacy_wireplumber_config(settings: &AudioSettings, stream_type: &str) -> Result<(), String> {
    println!("Applying legacy WirePlumber Lua configuration for version < 0.5");
    
    let config_content = generate_legacy_wireplumber_config(settings, stream_type);
    let username = whoami::username();
    let config_path = if stream_type == "output" {
        format!("/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-output.lua", username)
    } else {
        format!("/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-input.lua", username)
    };

    // Write configuration file (user path, no privileges needed)
    write_config_with_privileges(&config_path, &config_content)?;

    println!("Legacy WirePlumber Lua configuration written successfully");
    println!("- {}", config_path);

    // Restart audio services using legacy method
    restart_audio_services(true, false)?;

    println!("Audio services restarted successfully");
    Ok(())
}

/// Generates legacy WirePlumber Lua configuration content for versions < 0.5
fn generate_legacy_wireplumber_config(settings: &AudioSettings, stream_type: &str) -> String {
    let device_pattern = if settings.device_id == "default" {
        "alsa.*".to_string()
    } else {
        settings.device_id.clone()
    };

    let audio_format = match settings.bit_depth {
        16 => "S16LE",
        24 => "S24LE", 
        32 => "S32LE",
        _ => "S24LE"
    };

    format!(
        r#"-- Pro Audio Config Legacy Lua Configuration
-- For WirePlumber versions < 0.5
-- Auto-generated for {} settings

alsa_monitor.rules = {{
  {{
    matches = {{
      {{
        {{ "device.name", "matches", "{}" }},
      }},
    }},
    apply_properties = {{
      ["audio.format"] = "{}",
      ["audio.rate"] = {},
      ["api.alsa.period-size"] = {},
      ["api.alsa.period-num"] = 2,
      ["api.alsa.headroom"] = 8192,
    }},
  }}
}}
"#,
        stream_type,
        device_pattern,
        audio_format,
        settings.sample_rate,
        settings.buffer_size
    )
}

#[allow(dead_code)] // Used
/// Generates modern WirePlumber JSON configuration content for versions >= 0.5
fn generate_wireplumber_config(settings: &AudioSettings, _stream_type: &str) -> String {
    let device_pattern = if settings.device_id == "default" {
        "~alsa.*".to_string()
    } else {
        settings.device_id.clone()
    };

    let audio_format = match settings.bit_depth {
        16 => "S16LE",
        24 => "S24LE", 
        32 => "S32LE",
        _ => "S24LE"
    };

    format!(
        r#"{{
  "alsa-monitor": {{
    "rules": [
      {{
        "matches": [
          {{
            "device.name": "{}"
          }}
        ],
        "actions": [
          {{
            "update-props": {{
              "audio.format": "{}",
              "audio.rate": {},
              "api.alsa.period-size": {},
              "api.alsa.period-num"] = 2,
              "api.alsa.headroom"] = 8192
            }}
          }}
        ]
      }}
    ]
  }}
}}"#,
        device_pattern,
        audio_format,
        settings.sample_rate,
        settings.buffer_size
    )
}

/// Main function to apply audio settings using multiple configuration approaches with fallbacks
pub fn update_audio_settings(settings: &AudioSettings, system_wide: bool) -> Result<(), String> {
    println!("Applying {} audio settings: {}Hz/{}bit/{} samples", 
             if system_wide { "system-wide" } else { "user" },
             settings.sample_rate, settings.bit_depth, settings.buffer_size);
    
    // Clean up conflicting configs first
    if system_wide {
        cleanup_user_pipewire_configs()?;
        create_system_pipewire_fragment(settings)?;
    } else {
        cleanup_system_pipewire_configs()?;
        create_user_pipewire_fragment(settings)?;
    }
    
    // Try multiple configuration approaches in order of preference
    let mut success = match create_pipewire_fragment(settings, system_wide) {
        Ok(_) => {
            println!("✓ Successfully created PipeWire config fragment");
            true
        }
        Err(e) => {
            println!("PipeWire fragment approach failed: {}, trying next approach...", e);
            false
        }
    };
    
    if !success {
        // Approach 2: Create WirePlumber config (if available)
        match create_wireplumber_config_new(settings, system_wide) {
            Ok(_) => {
                println!("✓ Successfully created WirePlumber config");
                success = true;
            }
            Err(e) => {
                println!("WirePlumber approach failed: {}, trying final approach...", e);
                
                // Approach 3: Modify main pipewire.conf as fallback
                match modify_main_pipewire_config(settings, system_wide) {
                    Ok(_) => {
                        println!("✓ Successfully modified main PipeWire config");
                        success = true;
                    }
                    Err(e) => {
                        return Err(format!("All configuration approaches failed: {}", e));
                    }
                }
            }
        }
    }
    
    if success {
        // Wait a bit for the config to be written
        std::thread::sleep(std::time::Duration::from_millis(500));
        restart_audio_services(false, system_wide)?;
        println!("✓ Audio services restarted successfully");
        
        // Verify the settings were applied
        verify_settings_applied(settings)?;
        
        Ok(())
    } else {
        Err("Failed to apply audio settings through any method".to_string())
    }
}

/// Creates a user-specific PipeWire configuration fragment and cleans up system configs
fn create_user_pipewire_fragment(settings: &AudioSettings) -> Result<(), String> {
    // Use user directories only
    let username = whoami::username();
    let config_dirs = [
        format!("/home/{}/.config/pipewire/pipewire.conf.d", username),
    ];

    let config_content = format!(
        r#"# Pro Audio Config - User High Priority Settings
# This file overrides default PipeWire settings

context.properties = {{
    default.clock.rate = {}
    default.clock.quantum = {}
    default.clock.allowed-rates = [ {} ]
    # Disable rate/quantum checking to ensure our settings are applied
    settings.check-quantum = false
    settings.check-rate = false
}}

context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -11
            rt.prio = 88
            rt.time.soft = 200000
            rt.time.hard = 200000
        }}
        flags = [ ifexists nofail ]
    }}
]"#,
        settings.sample_rate,
        settings.buffer_size,
        settings.sample_rate
    );

    for dir in &config_dirs {
        let config_path = format!("{}/99-pro-audio-high-priority.conf", dir);
        
        // Write file (user path, no privileges needed)
        write_config_with_privileges(&config_path, &config_content)?;
        println!("✓ User PipeWire config created: {}", config_path);
        cleanup_old_pipewire_configs(&dir)?;
        
        // CLEAN UP SYSTEM CONFIGS WHEN USING USER CONFIG
        cleanup_system_pipewire_configs()?;
        
        return Ok(());
    }

    Err("Failed to write user PipeWire configuration".to_string())
}

/// Creates a system-wide PipeWire configuration fragment and cleans up user configs
fn create_system_pipewire_fragment(settings: &AudioSettings) -> Result<(), String> {
    // Use system directories only
    let config_dirs = [
        "/etc/pipewire/pipewire.conf.d".to_string(),
    ];

    let config_content = format!(
        r#"# Pro Audio Config - System-wide High Priority Settings
# This file overrides default PipeWire settings

context.properties = {{
    default.clock.rate = {}
    default.clock.quantum = {}
    default.clock.allowed-rates = [ {} ]
    # Disable rate/quantum checking to ensure our settings are applied
    settings.check-quantum = false
    settings.check-rate = false
}}

context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -11
            rt.prio = 88
            rt.time.soft = 200000
            rt.time.hard = 200000
        }}
        flags = [ ifexists nofail ]
    }}
]"#,
        settings.sample_rate,
        settings.buffer_size,
        settings.sample_rate
    );

    for dir in &config_dirs {
        let config_path = format!("{}/99-pro-audio-high-priority.conf", dir);
        
        // Write file with privileges for system paths
        write_config_with_privileges(&config_path, &config_content)?;
        println!("✓ System PipeWire config created: {}", config_path);
            
        // Also remove any lower priority configs we might have created
        cleanup_old_pipewire_configs(&dir)?;
        
        // Clean up user configs when using system config  
        cleanup_user_pipewire_configs()?;
        
        return Ok(());
    }
    
    Err("Failed to write system PipeWire configuration".to_string())
}

/// Creates a PipeWire configuration fragment file with proper privilege handling
fn create_pipewire_fragment(settings: &AudioSettings, system_wide: bool) -> Result<(), String> {
    let config_content = format!(
        r#"# Pro Audio Config - High Priority Settings
# This file overrides default PipeWire settings

context.properties = {{
    default.clock.rate = {}
    default.clock.quantum = {}
    default.clock.allowed-rates = [ {} ]
    # Disable rate/quantum checking to ensure our settings are applied
    settings.check-quantum = false
    settings.check-rate = false
}}

context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -11
            rt.prio = 88
            rt.time.soft = 200000
            rt.time.hard = 200000
        }}
        flags = [ ifexists nofail ]
    }}
]"#,
        settings.sample_rate,
        settings.buffer_size,
        settings.sample_rate
    );

    // Try multiple standard locations - use higher number for higher priority
    let username = whoami::username();
    let config_dirs = if system_wide {
        vec!["/etc/pipewire/pipewire.conf.d".to_string()]
    } else {
        vec![format!("/home/{}/.config/pipewire/pipewire.conf.d", username)]
    };

    for dir in &config_dirs {
        let config_path = format!("{}/99-pro-audio-high-priority.conf", dir);
        
        // Write file with proper privilege handling
        write_config_with_privileges(&config_path, &config_content)?;
        println!("✓ PipeWire config created: {}", config_path);
        
        // Also remove any lower priority configs we might have created
        cleanup_old_pipewire_configs(&dir)?;
        
        return Ok(());
    }

    Err("Failed to write PipeWire configuration to any location".to_string())
}

/// Clean up old pipewire config files to avoid conflicts
fn cleanup_old_pipewire_configs(dir: &str) -> Result<(), String> {
    let old_configs = [
        format!("{}/99-pro-audio.conf", dir),
        format!("{}/50-pro-audio.conf", dir),
    ];
    
    for old_config in &old_configs {
        if Path::new(old_config).exists() {
            if old_config.starts_with("/etc/") {
                // System path - need privileges
                let _ = execute_with_privileges("rm", &["-f", old_config]);
            } else {
                // User path - no privileges needed
                let _ = fs::remove_file(old_config);
            }
        }
    }
    
    Ok(())
}

/// Clean up system-wide pipewire config files
fn cleanup_system_pipewire_configs() -> Result<(), String> {
    let system_configs = [
        "/etc/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
        "/etc/pipewire/pipewire.conf.d/99-pro-audio.conf",
        "/etc/pipewire/pipewire.conf.d/50-pro-audio.conf",
    ];
    
    let mut removed_count = 0;
    for config_path in &system_configs {
        if Path::new(config_path).exists() {
            match execute_with_privileges("rm", &["-f", config_path]) {
                Ok(_) => {
                    println!("✓ Removed system config: {}", config_path);
                    removed_count += 1;
                }
                Err(e) => {
                    println!("Note: Could not remove system config {}: {}", config_path, e);
                }
            }
        }
    }
    
    if removed_count > 0 {
        println!("✓ Removed {} system configuration files", removed_count);
    }
    
    Ok(())
}

/// Clean up user-specific pipewire config files  
fn cleanup_user_pipewire_configs() -> Result<(), String> {
    let username = whoami::username();
    let user_configs = [
        format!("/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf", username),
        format!("/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio.conf", username),
        format!("/home/{}/.config/pipewire/pipewire.conf.d/50-pro-audio.conf", username),
    ];
    
    let mut removed_count = 0;
    for config_path in &user_configs {
        if Path::new(config_path).exists() {
            match fs::remove_file(config_path) {
                Ok(_) => {
                    println!("✓ Removed user config: {}", config_path);
                    removed_count += 1;
                }
                Err(e) => {
                    println!("Note: Could not remove user config {}: {}", config_path, e);
                }
            }
        }
    }
    
    if removed_count > 0 {
        println!("✓ Removed {} user configuration files", removed_count);
    }
    
    Ok(())
}

/// Creates a WirePlumber configuration file (updated format for versions >= 0.5)
fn create_wireplumber_config_new(settings: &AudioSettings, system_wide: bool) -> Result<(), String> {
    let username = whoami::username();
    
    // Use consistent types - both vectors should contain String
    let config_dirs = if system_wide {
        vec!["/etc/wireplumber/wireplumber.conf.d".to_string()]
    } else {
        vec![format!("/home/{}/.config/wireplumber/wireplumber.conf.d", username)]
    };
    
    for dir in &config_dirs {
        // Create directory if it doesn't exist
        create_dir_all_with_privileges(&dir)?;
        
        let config_path = format!("{}/99-pro-audio.conf", dir);
        
        let content = format!(
            r#"{{
  "monitor.alsa.rules": [
    {{
      "matches": [
        {{
          "node.name": "~alsa.*"
        }}
      ],
      "actions": {{
        "update-props": {{
          "audio.rate": {},
          "audio.allowed-rates": [ {} ],
          "api.alsa.period-size": {}
        }}
      }}
    }}
  ]
}}"#,
            settings.sample_rate,
            settings.sample_rate, // Single allowed rate for simplicity
            settings.buffer_size
        );
        
        write_config_with_privileges(&config_path, &content)?;
        println!("Created WirePlumber config: {}", config_path);
        return Ok(());
    }
    
    Err("No WirePlumber config directory found".to_string())
}

/// Modifies the main PipeWire configuration file as a fallback
fn modify_main_pipewire_config(settings: &AudioSettings, system_wide: bool) -> Result<(), String> {
    let username = whoami::username();
    
    // Use consistent types - both vectors should contain String
    let config_paths = if system_wide {
        vec!["/etc/pipewire/pipewire.conf".to_string()]
    } else {
        vec![format!("/home/{}/.config/pipewire/pipewire.conf", username)]
    };
    
    for path in &config_paths {
        if Path::new(path).exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read {}: {}", path, e))?;
            
            // Try multiple possible patterns for clock rate and quantum
            let patterns = [
                ("#default.clock.rate          = 48000", format!("default.clock.rate          = {}", settings.sample_rate)),
                ("#default.clock.rate          = 192000", format!("default.clock.rate          = {}", settings.sample_rate)),
                ("default.clock.rate          = 48000", format!("default.clock.rate          = {}", settings.sample_rate)),
                ("#default.clock.quantum       = 1024", format!("default.clock.quantum       = {}", settings.buffer_size)),
                ("#default.clock.quantum       = 8192", format!("default.clock.quantum       = {}", settings.buffer_size)),
                ("default.clock.quantum       = 1024", format!("default.clock.quantum       = {}", settings.buffer_size)),
                // Also update the allowed rates
                ("default.clock.allowed-rates = \\[ 44100, 48000, 96000, 192000 \\]", format!("default.clock.allowed-rates = [ {} ]", settings.sample_rate)),
            ];
            
            let mut updated_content = content.clone();
            for (pattern, replacement) in &patterns {
                updated_content = updated_content.replace(pattern, replacement);
            }
            
            // Only write if changes were made
            if updated_content != content {
                // Backup original
                let backup_path = format!("{}.backup", path);
                if system_wide {
                    execute_with_privileges("cp", &[path, &backup_path])?;
                    write_config_with_privileges(path, &updated_content)?;
                } else {
                    fs::copy(path, &backup_path)
                        .map_err(|e| format!("Failed to backup {}: {}", path, e))?;
                    fs::write(path, updated_content)
                        .map_err(|e| format!("Failed to write {}: {}", path, e))?;
                }
                
                println!("Modified main config: {} (backup: {})", path, backup_path);
            } else {
                println!("No changes needed for: {}", path);
            }
            
            return Ok(());
        }
    }
    
    Err("No main PipeWire config file found".to_string())
}

/// Verifies that the settings were actually applied
fn verify_settings_applied(settings: &AudioSettings) -> Result<(), String> {
    println!("Verifying settings were applied...");
    
    // Wait a bit for services to fully initialize
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Get current settings
    let current_settings = crate::audio::detect_current_audio_settings()
        .map_err(|e| format!("Failed to detect current settings for verification: {}", e))?;
    
    println!("Current settings: {}Hz/{}bit/{} samples", 
             current_settings.sample_rate, current_settings.bit_depth, current_settings.buffer_size);
    println!("Expected settings: {}Hz/{}bit/{} samples", 
             settings.sample_rate, settings.bit_depth, settings.buffer_size);
    
    // Check if settings match (with some tolerance for buffer size)
    if current_settings.sample_rate != settings.sample_rate {
        return Err(format!("Sample rate not applied: expected {}Hz, got {}Hz", 
                          settings.sample_rate, current_settings.sample_rate));
    }
    
    // Buffer size might be adjusted by PipeWire, so we allow some difference
    let buffer_diff = (current_settings.buffer_size as i32 - settings.buffer_size as i32).abs();
    if buffer_diff > 256 { // Allow 256 samples difference
        return Err(format!("Buffer size significantly different: expected {}, got {}", 
                          settings.buffer_size, current_settings.buffer_size));
    }
    
    println!("✓ Settings verified successfully");
    Ok(())
}

pub fn debug_wireplumber_config() -> Result<(), String> {
    println!("=== WirePlumber Debug Information ===");
    
    // Check if WirePlumber is running
    let status = Command::new("systemctl")
        .args(["--user", "is-active", "wireplumber"])
        .status()
        .map_err(|e| format!("Failed to check WirePlumber status: {}", e))?;
    
    if !status.success() {
        return Err("WirePlumber is not running".to_string());
    }
    
    // Get and display WirePlumber version
    match get_wireplumber_version() {
        Ok(version) => println!("WirePlumber version: {}", version),
        Err(e) => println!("Could not determine WirePlumber version: {}", e),
    }
    
    // List config files WirePlumber is using
    let _ = Command::new("wpctl")
        .args(["status"])
        .status();
    
    // Check config directory structure
    let username = whoami::username();
    let config_dirs = [
        format!("/home/{}/.config/wireplumber", username),
        "/etc/wireplumber".to_string(),
    ];
    
    for dir in &config_dirs {
        println!("Checking directory: {}", dir);
        if Path::new(dir).exists() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    println!("  Found: {}", entry.path().display());
                }
            }
        }
    }
    
    Ok(())
}

/// Unified function to restart audio services with system-wide support
fn restart_audio_services(use_legacy: bool, system_wide: bool) -> Result<(), String> {
    println!("Restarting audio services...");
    
    let username = whoami::username();

    if system_wide {
        // For system-wide config changes, restart the user services for the target user.
        println!("Restarting user audio services for user '{}' with privileges...", username);
        
        // Use `sudo -u $username` to run the user service commands as that specific user.
        let status = Command::new("sudo")
            .args(["-u", &username, "systemctl", "--user", "restart", "pipewire", "pipewire-pulse", "wireplumber"])
            .status()
            .map_err(|e| format!("Failed to restart user services: {}", e))?;
        
        if !status.success() {
            return Err("Failed to restart user audio services via systemd".to_string());
        }
    } else {
        // User-only restart
        if use_legacy {
            // Legacy approach: Restart services individually with pauses
            let services = ["pipewire", "pipewire-pulse", "wireplumber"];
            
            for service in &services {
                let status = Command::new("systemctl")
                    .args(["--user", "restart", service])
                    .status()
                    .map_err(|e| format!("Failed to restart {}: {}", service, e))?;
                
                if !status.success() {
                    return Err(format!("Failed to restart {}", service));
                }
                
                // Brief pause between service restarts
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        } else {
            // New approach: Use systemd if available, otherwise direct commands
            if Path::new("/run/systemd/seats").exists() {
                println!("Using systemd to restart user services...");
                let status = Command::new("systemctl")
                    .args(["--user", "restart", "pipewire", "pipewire-pulse", "wireplumber"])
                    .status()
                    .map_err(|e| format!("Failed to restart services: {}", e))?;
                
                if !status.success() {
                    return Err("Failed to restart audio services via systemd".to_string());
                }
            } else {
                // Fallback: kill and let them restart automatically
                println!("Systemd not available, using fallback restart method...");
                Command::new("pkill")
                    .args(["-f", "pipewire"])
                    .status()
                    .ok(); // Ignore errors here
                
                Command::new("pkill") 
                    .args(["-f", "wireplumber"])
                    .status()
                    .ok();
            }
        }
        
        // Wait a moment for services to restart
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    // Additional wait for services to fully initialize
    println!("Waiting for services to initialize...");
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    Ok(())
}

/// Removes any configuration files created by this application
pub fn cleanup_config_files() -> Result<(), String> {
    let username = whoami::username();
    let files_to_remove = [
        "/etc/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
        "/etc/pipewire/pipewire.conf.d/99-pro-audio.conf",
        &format!("/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf", username),
        &format!("/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio.conf", username),
        &format!("/home/{}/.local/share/pipewire/pipewire.conf.d/99-pro-audio.conf", username),
        "/etc/wireplumber/wireplumber.conf.d/99-pro-audio.conf", 
        "/etc/wireplumber/wireplumber.conf.d/50-pro-audio.conf", 
        &format!("/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio.conf", username),
        &format!("/home/{}/.config/wireplumber/wireplumber.conf.d/50-pro-audio.conf", username),
        &format!("/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-output.lua", username),
        &format!("/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-input.lua", username),
    ];
    
    let mut removed_count = 0;
    for file in &files_to_remove {
        let expanded_path = shellexpand::full(file).map_err(|e| e.to_string())?;
        if Path::new(&*expanded_path).exists() {
            if expanded_path.starts_with("/etc/") {
                // System path - need privileges
                if execute_with_privileges("rm", &["-f", &expanded_path]).is_ok() {
                    println!("Removed system config: {}", expanded_path);
                    removed_count += 1;
                }
            } else {
                // User path - no privileges needed
                match fs::remove_file(&*expanded_path) {
                    Ok(_) => {
                        println!("Removed config file: {}", expanded_path);
                        removed_count += 1;
                    }
                    Err(e) => {
                        println!("Warning: Failed to remove {}: {}", expanded_path, e);
                    }
                }
            }
        }
    }
    
    if removed_count > 0 {
        restart_audio_services(false, false)?;
        println!("Removed {} configuration files and restarted services", removed_count);
    }
    
    Ok(())
}

/// Checks if audio services are running
pub fn check_audio_services() -> Result<(), String> {
    let services = ["pipewire", "pipewire-pulse", "wireplumber"];
    let mut missing_services = Vec::new();
    
    for service in &services {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(service)
            .output()
            .map_err(|e| format!("Failed to check service {}: {}", service, e))?;
        
        if !output.status.success() {
            missing_services.push(service.to_string());
        }
    }
    
    if !missing_services.is_empty() {
        Err(format!("Missing audio services: {}", missing_services.join(", ")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_settings_struct() {
        let settings = AudioSettings {
            sample_rate: 96000,
            bit_depth: 24,
            buffer_size: 512,
            device_id: "test-device".to_string(),
        };
        
        assert_eq!(settings.sample_rate, 96000);
        assert_eq!(settings.bit_depth, 24);
        assert_eq!(settings.buffer_size, 512);
    }
    
    #[test]
    fn test_wireplumber_config_generation() {
        let settings = AudioSettings {
            sample_rate: 192000,
            bit_depth: 32,
            buffer_size: 256,
            device_id: "test-device".to_string(),
        };
        
        let config = generate_wireplumber_config(&settings, "output");
        
        assert!(config.contains("192000"));
        assert!(config.contains("256"));
        assert!(config.contains("S32LE"));
        assert!(config.contains("test-device"));
    }
    
    #[test]
    fn test_legacy_wireplumber_config_generation() {
        let settings = AudioSettings {
            sample_rate: 48000,
            bit_depth: 16,
            buffer_size: 1024,
            device_id: "default".to_string(),
        };
        
        let config = generate_legacy_wireplumber_config(&settings, "input");
        
        assert!(config.contains("48000"));
        assert!(config.contains("1024")); 
        assert!(config.contains("alsa.*"));
        assert!(config.contains("S16LE"));
        assert!(config.contains("Lua"));
    }
    
    #[test]
    fn test_default_device_config() {
        let settings = AudioSettings {
            sample_rate: 48000,
            bit_depth: 16,
            buffer_size: 1024,
            device_id: "default".to_string(),
        };
        
        let config = generate_wireplumber_config(&settings, "input");
        
        assert!(config.contains("48000"));
        assert!(config.contains("1024")); 
        assert!(config.contains("~alsa.*"));
        assert!(config.contains("S16LE"));
    }
}
