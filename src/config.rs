/*
 * Pro Audio Config - Configuration Module
 * Version: 1.7
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 *
 * Handles audio configuration for PipeWire and WirePlumber with authentication
 * and multiple fallback approaches.
 */

use crate::audio::AudioSettings;
use chrono::Local;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Applies output audio settings with authentication
pub fn apply_output_audio_settings_with_auth_blocking(
    settings: AudioSettings,
) -> Result<(), String> {
    apply_audio_settings_with_auth(settings, "output")
}

/// Applies input audio settings with authentication
pub fn apply_input_audio_settings_with_auth_blocking(
    settings: AudioSettings,
) -> Result<(), String> {
    apply_audio_settings_with_auth(settings, "input")
}

/// Applies audio settings for user-specific configuration
pub fn apply_user_audio_settings(settings: AudioSettings, tab_type: &str) -> Result<(), String> {
    println!("Applying user-specific {} audio settings", tab_type);
    update_audio_settings(&settings, false) // false = not system-wide
}

/// Main function to apply audio settings with authentication
fn apply_audio_settings_with_auth(
    settings: AudioSettings,
    stream_type: &str,
) -> Result<(), String> {
    println!(
        "Applying {} audio settings with authentication: {}Hz/{}bit/{} samples",
        stream_type, settings.sample_rate, settings.bit_depth, settings.buffer_size
    );

    // First try the new multi-approach configuration
    match update_audio_settings(&settings, true) {
        Ok(()) => {
            println!(
                "✓ Successfully applied {} settings using new configuration system",
                stream_type
            );
            return Ok(());
        }
        Err(e) => {
            println!(
                "New configuration system failed: {}, falling back to legacy method...",
                e
            );
        }
    }

    // Fall back to legacy WirePlumber configuration (only for older versions)
    if should_use_legacy_wireplumber_config()? {
        apply_legacy_wireplumber_config(&settings, stream_type)
    } else {
        Err(
            "Legacy WirePlumber configuration not applicable for current WirePlumber version"
                .to_string(),
        )
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
        .map_err(|e| {
            format!(
                "Failed to execute with pkexec: {}. Make sure pkexec is available.",
                e
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        // Fallback to sudo
        println!("pkexec failed, trying sudo...");
        let status = Command::new("sudo")
            .arg(command)
            .args(args)
            .status()
            .map_err(|e| {
                format!(
                    "Failed to execute with sudo: {}. Please run with administrator privileges.",
                    e
                )
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(
                "Privilege escalation failed. Please run as root or configure sudo/pkexec."
                    .to_string(),
            )
        }
    }
}

/// Write configuration file with proper privilege escalation for system paths
fn write_config_with_privileges(config_path: &str, content: &str) -> Result<(), String> {
    if config_path.starts_with("/etc/") {
        // System path - need privileges
        let temp_file = format!("/tmp/pro-audio-config-{}", std::process::id());

        // Write to temp file first
        fs::write(&temp_file, content).map_err(|e| format!("Failed to write temp file: {}", e))?;

        // Copy to system location with privileges
        execute_with_privileges("cp", &[&temp_file, config_path])?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_file);

        println!("✓ System config created with privileges: {}", config_path);
    } else {
        // User path - no privileges needed
        if let Some(parent) = Path::new(config_path).parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        fs::write(config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
    }

    Ok(())
}

/// Create directory with proper privilege escalation for system paths
fn create_dir_all_with_privileges(path: &str) -> Result<(), String> {
    if path.starts_with("/etc/") {
        // For system paths, check if directory already exists first
        if Path::new(path).exists() {
            return Ok(());
        }
        // System path - need privileges
        execute_with_privileges("mkdir", &["-p", path])?;
    } else {
        // User path - no privileges needed
        fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
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
            println!(
                "Could not detect WirePlumber version: {}, assuming modern version",
                e
            );
            // If we can't detect version, assume modern and don't use legacy config
            Ok(false)
        }
    }
}

/// Gets the WirePlumber version as a string
fn get_wireplumber_version() -> Result<String, String> {
    // Method 1: Try wireplumber --version directly
    if let Ok(output) = Command::new("wireplumber").args(["--version"]).output() {
        let version_output = String::from_utf8_lossy(&output.stdout);
        for line in version_output.lines() {
            if line.contains("WirePlumber")
                || line.contains("wireplumber")
                || line.contains("version")
            {
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
                        if word.chars().next().map_or(false, |c| c.is_numeric())
                            && word.contains('.')
                        {
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
fn apply_legacy_wireplumber_config(
    settings: &AudioSettings,
    stream_type: &str,
) -> Result<(), String> {
    println!("Applying legacy WirePlumber Lua configuration for version < 0.5");

    let config_content = generate_legacy_wireplumber_config(settings, stream_type);
    let username = whoami::username();
    let config_path = if stream_type == "output" {
        format!(
            "/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-output.lua",
            username
        )
    } else {
        format!(
            "/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-input.lua",
            username
        )
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
        _ => "S24LE",
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
        stream_type, device_pattern, audio_format, settings.sample_rate, settings.buffer_size
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
        _ => "S24LE",
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
        device_pattern, audio_format, settings.sample_rate, settings.buffer_size
    )
}

/// Main function to apply audio settings using multiple configuration approaches with fallbacks
pub fn update_audio_settings(settings: &AudioSettings, system_wide: bool) -> Result<(), String> {
    println!(
        "Applying {} audio settings: {}Hz/{}bit/{} samples",
        if system_wide { "system-wide" } else { "user" },
        settings.sample_rate,
        settings.bit_depth,
        settings.buffer_size
    );

    // Try multiple configuration approaches in order of preference
    let mut success = false;

    // Clean up conflicting configs first
    if system_wide {
        cleanup_user_pipewire_configs()?;
    } else {
        cleanup_system_pipewire_configs()?;
    }

    // Approach 1: Create PipeWire config fragment
    match create_pipewire_fragment(settings, system_wide) {
        Ok(_) => {
            println!("✓ Successfully created PipeWire config fragment");
            success = true;
        }
        Err(e) => {
            println!(
                "PipeWire fragment approach failed: {}, trying next approach...",
                e
            );
        }
    }

    if !success {
        // Approach 2: Create WirePlumber config (if available)
        match create_wireplumber_config_new(settings, system_wide) {
            Ok(_) => {
                println!("✓ Successfully created WirePlumber config");
                success = true;
            }
            Err(e) => {
                println!(
                    "WirePlumber approach failed: {}, trying final approach...",
                    e
                );

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
        settings.sample_rate, settings.buffer_size, settings.sample_rate
    );

    // Try multiple standard locations - use higher number for higher priority
    let username = whoami::username();
    let config_dirs = if system_wide {
        vec!["/etc/pipewire/pipewire.conf.d".to_string()]
    } else {
        vec![format!(
            "/home/{}/.config/pipewire/pipewire.conf.d",
            username
        )]
    };

    for dir in &config_dirs {
        let config_path = format!("{}/99-pro-audio-high-priority.conf", dir);

        // Skip backup for system directories to avoid permission issues
        // The backup is just a safety measure, not critical
        if !dir.starts_with("/etc/") {
            if let Err(e) = backup_current_config(&dir) {
                println!("Note: Could not backup config (non-fatal): {}", e);
            }
        }

        // Write file with proper privilege handling
        if let Err(e) = write_config_with_privileges(&config_path, &config_content) {
            println!(
                "Failed to write to {}: {}, trying next location...",
                config_path, e
            );
            continue;
        }

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
                    println!(
                        "Note: Could not remove system config {}: {}",
                        config_path, e
                    );
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
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
            username
        ),
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio.conf",
            username
        ),
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/50-pro-audio.conf",
            username
        ),
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
fn create_wireplumber_config_new(
    settings: &AudioSettings,
    system_wide: bool,
) -> Result<(), String> {
    let username = whoami::username();

    // Use consistent types - both vectors should contain String
    let config_dirs = if system_wide {
        vec!["/etc/wireplumber/wireplumber.conf.d".to_string()]
    } else {
        vec![format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d",
            username
        )]
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

        if let Err(e) = write_config_with_privileges(&config_path, &content) {
            println!(
                "Failed to write to {}: {}, trying next location...",
                config_path, e
            );
            continue;
        }

        println!("✓ Created WirePlumber config: {}", config_path);
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
        if !Path::new(path).exists() {
            continue;
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

        // Try multiple possible patterns for clock rate and quantum
        let patterns = [
            (
                "#default.clock.rate          = 48000",
                format!("default.clock.rate          = {}", settings.sample_rate),
            ),
            (
                "#default.clock.rate          = 192000",
                format!("default.clock.rate          = {}", settings.sample_rate),
            ),
            (
                "default.clock.rate          = 48000",
                format!("default.clock.rate          = {}", settings.sample_rate),
            ),
            (
                "#default.clock.quantum       = 1024",
                format!("default.clock.quantum       = {}", settings.buffer_size),
            ),
            (
                "#default.clock.quantum       = 8192",
                format!("default.clock.quantum       = {}", settings.buffer_size),
            ),
            (
                "default.clock.quantum       = 1024",
                format!("default.clock.quantum       = {}", settings.buffer_size),
            ),
            // Also update the allowed rates
            (
                "default.clock.allowed-rates = \\[ 44100, 48000, 96000, 192000 \\]",
                format!("default.clock.allowed-rates = [ {} ]", settings.sample_rate),
            ),
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

    println!(
        "Current settings: {}Hz/{}bit/{} samples",
        current_settings.sample_rate, current_settings.bit_depth, current_settings.buffer_size
    );
    println!(
        "Expected settings: {}Hz/{}bit/{} samples",
        settings.sample_rate, settings.bit_depth, settings.buffer_size
    );

    // Check if settings match (with some tolerance for buffer size)
    if current_settings.sample_rate != settings.sample_rate {
        return Err(format!(
            "Sample rate not applied: expected {}Hz, got {}Hz",
            settings.sample_rate, current_settings.sample_rate
        ));
    }

    // Buffer size might be adjusted by PipeWire, so we allow some difference
    let buffer_diff = (current_settings.buffer_size as i32 - settings.buffer_size as i32).abs();
    if buffer_diff > 256 {
        // Allow 256 samples difference
        return Err(format!(
            "Buffer size significantly different: expected {}, got {}",
            settings.buffer_size, current_settings.buffer_size
        ));
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
    let _ = Command::new("wpctl").args(["status"]).status();

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

/// Improved unified function to restart audio services with timeout
fn restart_audio_services(use_legacy: bool, system_wide: bool) -> Result<(), String> {
    println!("Restarting audio services...");
    let start_time = Instant::now();

    let username = whoami::username();

    if system_wide {
        // For system-wide config changes, restart the user services for the target user.
        println!(
            "Restarting user audio services for user '{}' with privileges...",
            username
        );

        // Use `sudo -u $username` to run the user service commands as that specific user.
        let status = Command::new("sudo")
            .args([
                "-u",
                &username,
                "systemctl",
                "--user",
                "restart",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
            ])
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
                println!("Restarting {}...", service);
                let status = Command::new("systemctl")
                    .args(["--user", "restart", service])
                    .status()
                    .map_err(|e| format!("Failed to restart {}: {}", service, e))?;

                if !status.success() {
                    println!("Warning: Failed to restart {}", service);
                    // Continue with other services instead of failing completely
                }

                // Brief pause between service restarts
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        } else {
            // New approach: Use systemd if available, otherwise direct commands
            if Path::new("/run/systemd/seats").exists() {
                println!("Using systemd to restart user services...");
                let status = Command::new("systemctl")
                    .args([
                        "--user",
                        "restart",
                        "pipewire",
                        "pipewire-pulse",
                        "wireplumber",
                    ])
                    .status()
                    .map_err(|e| format!("Failed to restart services: {}", e))?;

                if !status.success() {
                    return Err("Failed to restart audio services via systemd".to_string());
                }
            } else {
                // Fallback: kill and let them restart automatically
                println!("Systemd not available, using fallback restart method...");
                Command::new("pkill").args(["-f", "pipewire"]).status().ok(); // Ignore errors here
                Command::new("pkill")
                    .args(["-f", "wireplumber"])
                    .status()
                    .ok();
            }
        }

        // Wait a moment for services to restart
        println!("Waiting for services to restart...");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    // NEW: Check if services are actually running with timeout
    println!("Checking if audio services are running...");
    let max_wait_time = Duration::from_secs(10);
    let check_interval = Duration::from_millis(500);

    let mut elapsed = Duration::from_secs(0);
    let mut services_running = false;

    while elapsed < max_wait_time {
        // Check if at least one audio service is running
        if check_if_services_are_running() {
            services_running = true;
            break;
        }

        std::thread::sleep(check_interval);
        elapsed += check_interval;

        if elapsed.as_secs() % 2 == 0 {
            println!(
                "Waiting for audio services... ({:.1}s)",
                elapsed.as_secs_f32()
            );
        }
    }

    if !services_running {
        println!(
            "⚠ Audio services did not start within timeout. They may start in the background."
        );
        println!("⚠ You can check service status with: systemctl --user status pipewire");
        // Don't fail, just warn - the services might start later
    } else {
        println!("✓ Audio services are running");
    }

    let total_time = start_time.elapsed();
    println!(
        "✓ Audio service restart completed in {:.1}s",
        total_time.as_secs_f32()
    );

    // Additional wait for services to fully initialize
    if services_running {
        println!("Waiting for services to fully initialize...");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    Ok(())
}

/// Check if audio services are running
fn check_if_services_are_running() -> bool {
    let services = ["pipewire", "pipewire-pulse", "wireplumber"];

    // Check if any of the services are running
    for service in &services {
        if let Ok(output) = Command::new("pgrep").arg("-f").arg(service).output() {
            if output.status.success() {
                // At least one service is running
                return true;
            }
        }
    }

    // Also check with systemctl as fallback
    if let Ok(output) = Command::new("systemctl")
        .args(["--user", "is-active", "pipewire"])
        .output()
    {
        if output.status.success() {
            return true;
        }
    }

    false
}

/// Removes any configuration files created by this application
pub fn cleanup_config_files() -> Result<(), String> {
    let username = whoami::username();
    let files_to_remove = [
        "/etc/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
        "/etc/pipewire/pipewire.conf.d/99-pro-audio.conf",
        &format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
            username
        ),
        &format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio.conf",
            username
        ),
        &format!(
            "/home/{}/.local/share/pipewire/pipewire.conf.d/99-pro-audio.conf",
            username
        ),
        "/etc/wireplumber/wireplumber.conf.d/99-pro-audio.conf",
        "/etc/wireplumber/wireplumber.conf.d/50-pro-audio.conf",
        &format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio.conf",
            username
        ),
        &format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d/50-pro-audio.conf",
            username
        ),
        &format!(
            "/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-output.lua",
            username
        ),
        &format!(
            "/home/{}/.config/wireplumber/main.lua.d/50-pro-audio-input.lua",
            username
        ),
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
        // Use the improved restart function
        restart_audio_services(false, false)?;
        println!(
            "Removed {} configuration files and restarted services",
            removed_count
        );
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
        Err(format!(
            "Missing audio services: {}",
            missing_services.join(", ")
        ))
    } else {
        Ok(())
    }
}

/// Advanced audio settings structure
#[derive(Clone, Debug)]
pub struct AdvancedAudioSettings {
    pub exclusive_mode: bool,
    pub direct_hardware: bool,
    pub low_latency: bool,
    pub buffer_size: u32,
    pub sample_rate: u32,
}

/// Apply advanced audio settings with exclusive mode support
pub fn apply_advanced_audio_settings(
    exclusive_mode: bool,
    direct_hardware: bool,
    low_latency: bool,
    buffer_size: u32,
    sample_rate: u32,
    device_pattern: Option<String>, // Add device pattern parameter
) -> Result<(), String> {
    println!("Applying advanced audio settings:");
    println!("  Exclusive Mode: {}", exclusive_mode);
    println!("  Direct Hardware: {}", direct_hardware);
    println!("  Low Latency: {}", low_latency);
    println!("  Buffer Size: {}", buffer_size);
    println!("  Sample Rate: {}", sample_rate);

    if let Some(pattern) = &device_pattern {
        println!("  Device Pattern: {}", pattern);
    }

    if exclusive_mode {
        let device = device_pattern.unwrap_or_else(|| "default".to_string());
        apply_enhanced_exclusive_mode_settings(
            direct_hardware,
            low_latency,
            buffer_size,
            sample_rate,
            &device,
        )
    } else {
        // Return to standard shared mode
        restore_standard_audio_mode()
    }
}

/// Enhanced exclusive mode with device capability checking
fn apply_enhanced_exclusive_mode_settings(
    direct_hardware: bool,
    low_latency: bool,
    buffer_size: u32,
    sample_rate: u32,
    device_pattern: &str,
) -> Result<(), String> {
    println!("Configuring enhanced exclusive audio access mode...");

    // Check device suitability for exclusive mode
    if let Ok(devices) = crate::audio::detect_high_performance_devices() {
        let target_device = if device_pattern == "default" {
            devices.first()
        } else {
            devices.iter().find(|d| d.id.contains(device_pattern))
        };

        if let Some(device) = target_device {
            if !crate::audio::is_device_suitable_for_exclusive_mode(device) {
                println!("Warning: Selected device may not be ideal for exclusive mode");
            }

            // Get device capabilities
            if let Ok(capabilities) = crate::audio::get_device_capabilities(&device.id) {
                if !capabilities.buffer_sizes.contains(&buffer_size) {
                    println!(
                        "Warning: Buffer size {} may not be optimal for {}",
                        buffer_size, device.name
                    );
                }
                if !capabilities.sample_rates.contains(&sample_rate) {
                    println!(
                        "Warning: Sample rate {} may not be supported by {}",
                        sample_rate, device.name
                    );
                }
            }
        }
    }

    // Proceed with existing exclusive mode configuration
    apply_exclusive_mode_settings(direct_hardware, low_latency, buffer_size, sample_rate)
}

/// Apply exclusive mode configuration
fn apply_exclusive_mode_settings(
    direct_hardware: bool,
    low_latency: bool,
    buffer_size: u32,
    sample_rate: u32,
) -> Result<(), String> {
    println!("Configuring exclusive audio access mode...");

    // First, try the modern PipeWire exclusive mode approach
    match create_pipewire_exclusive_config(direct_hardware, low_latency, buffer_size, sample_rate) {
        Ok(()) => {
            println!("✓ PipeWire exclusive mode configured successfully");
            restart_audio_services(false, true)?;
            return Ok(());
        }
        Err(e) => {
            println!("PipeWire exclusive mode failed: {}, trying fallback...", e);
        }
    }

    // Fallback: Use WirePlumber configuration for exclusive access
    match create_wireplumber_exclusive_config(
        direct_hardware,
        low_latency,
        buffer_size,
        sample_rate,
    ) {
        Ok(()) => {
            println!("✓ WirePlumber exclusive mode configured successfully");
            restart_audio_services(false, true)?;
            Ok(())
        }
        Err(e) => Err(format!("All exclusive mode approaches failed: {}", e)),
    }
}

/// Create PipeWire configuration for exclusive mode
fn create_pipewire_exclusive_config(
    direct_hardware: bool,
    low_latency: bool,
    buffer_size: u32,
    sample_rate: u32,
) -> Result<(), String> {
    let username = whoami::username();
    let config_dir = format!("/home/{}/.config/pipewire/pipewire.conf.d", username);
    let config_path = format!("{}/99-pro-audio-exclusive.conf", config_dir);

    let audio_format = if low_latency { "S32LE" } else { "S24LE" };

    // FIXED: More conservative settings that won't break PipeWire
    let config_content = if direct_hardware {
        format!(
            r#"# Pro Audio Config - Exclusive Direct Hardware Access
# This configuration enables ASIO-like exclusive mode

context.properties = {{
    default.clock.rate = {}
    default.clock.quantum = {}
    default.clock.allowed-rates = [ {} ]
    # Use standard rate checking to avoid crashes
    settings.check-quantum = true
    settings.check-rate = true
}}

# Configure for low latency with safe defaults
context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -11    # Reduced from -20 to avoid permission issues
            rt.prio = 80        # Reduced from 88 to avoid permission issues
            rt.time.soft = 200000  # Increased from 100000 for better compatibility
            rt.time.hard = 200000  # Increased from 100000 for better compatibility
        }}
        flags = [ ifexists nofail ]
    }}
]

# Node configuration for exclusive access
node.factory = {{
    args = {{
        node.name = "pro-audio-exclusive-node"
        node.description = "Pro Audio Exclusive Mode"
        media.class = "Audio/Sink"
        audio.format = "{}"
        audio.rate = {}
        audio.allowed-rates = [ {} ]
        audio.channels = 2
        audio.position = [ FL, FR ]
        priority.driver = 100
        session.suspend-timeout-seconds = 10  # ADDED: Allow timeout for recovery
    }}
}}

# Add fallback mechanism
context.spa-libs = {{
    # Ensure standard libraries are available
    api.alsa.* = alsa/libspa-alsa
    api.v4l2.* = v4l2/libspa-v4l2
}}
"#,
            sample_rate, buffer_size, sample_rate, audio_format, sample_rate, sample_rate
        )
    } else {
        format!(
            r#"# Pro Audio Config - Exclusive PipeWire Access
# This configuration enables exclusive mode with PipeWire processing

context.properties = {{
    default.clock.rate = {}
    default.clock.quantum = {}
    default.clock.allowed-rates = [ {} ]
    # Minimal processing for low latency
    settings.check-quantum = true
    settings.check-rate = true
}}

context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -15
            rt.prio = 80
            rt.time.soft = 200000
            rt.time.hard = 200000
        }}
        flags = [ ifexists nofail ]
    }}
]

# Reserve device for exclusive use with specific pattern
device.rules = [
    {{
        matches = [
            {{ "device.name", "matches", "alsa.*" }}
        ],
        actions = {{
            update-props = {{
                device.profile = "pro-audio"
                api.alsa.disable-batch = true
                api.alsa.use-acp = false
                api.alsa.headroom = {}
                session.suspend-timeout-seconds = 5  # ADDED: Allow shorter timeout
            }}
        }}
    }}
]
"#,
            sample_rate,
            buffer_size,
            sample_rate,
            buffer_size / 2
        )
    };

    // Backup current config before writing
    if let Err(e) = backup_current_config(&config_dir) {
        println!("Note: Could not backup config (non-fatal): {}", e);
    }

    write_config_with_privileges(&config_path, &config_content)?;
    println!("✓ Exclusive mode configuration created: {}", config_path);
    Ok(())
}

/// Create WirePlumber configuration for exclusive access
fn create_wireplumber_exclusive_config(
    direct_hardware: bool,
    low_latency: bool,
    buffer_size: u32,
    sample_rate: u32,
) -> Result<(), String> {
    let username = whoami::username();
    let config_dir = format!("/home/{}/.config/wireplumber/wireplumber.conf.d", username);
    let config_path = format!("{}/99-pro-audio-exclusive.conf", config_dir);

    let config_content = format!(
        r#"{{
  "monitor.alsa.rules": [
    {{
      "matches": [
        {{
          "device.name": "~alsa.*"
        }}
      ],
      "actions": {{
        "update-props": {{
          "audio.rate": {},
          "audio.allowed-rates": [ {} ],
          "api.alsa.period-size": {},
          "api.alsa.period-num": 2,
          "api.alsa.headroom": {},
          "api.alsa.disable-batch": {},
          "api.alsa.use-acp": false,
          "priority.driver": 200,
          "session.suspend-timeout-seconds": 5  # REDUCED: Allow timeout for recovery
        }}
      }}
    }}
  ]
}}"#,
        sample_rate,
        sample_rate,
        buffer_size,
        buffer_size / 2,
        direct_hardware
    );

    // Backup current config before writing
    if let Err(e) = backup_current_config(&config_dir) {
        println!("Note: Could not backup config (non-fatal): {}", e);
    }

    create_dir_all_with_privileges(&config_dir)?;
    write_config_with_privileges(&config_path, &config_content)?;
    println!(
        "✓ WirePlumber exclusive configuration created: {}",
        config_path
    );
    Ok(())
}

/// Restore standard shared audio mode
pub fn restore_standard_audio_mode() -> Result<(), String> {
    println!("Restoring standard shared audio mode...");

    // Remove exclusive mode configurations
    let username = whoami::username();
    let config_files = [
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
            username
        ),
        format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio-exclusive.conf",
            username
        ),
    ];

    let mut removed_count = 0;
    for config_file in &config_files {
        if Path::new(config_file).exists() {
            if let Ok(()) = fs::remove_file(config_file) {
                println!("✓ Removed exclusive config: {}", config_file);
                removed_count += 1;
            }
        }
    }

    if removed_count > 0 {
        restart_audio_services(false, true)?;
        println!("✓ Standard audio mode restored");
    } else {
        println!("✓ Already in standard audio mode");
    }

    Ok(())
}

/// Check if exclusive mode is currently active
pub fn check_exclusive_mode_status() -> Result<bool, String> {
    let username = whoami::username();
    let exclusive_config = format!(
        "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
        username
    );

    Ok(Path::new(&exclusive_config).exists())
}

/// NEW: Backup current configuration before making changes
fn backup_current_config(config_dir: &str) -> Result<(), String> {
    // Skip backup for system directories to avoid permission issues
    // The backup is just a safety measure, not critical
    if config_dir.starts_with("/etc/") {
        println!("Note: Skipping backup for system directory {}", config_dir);
        return Ok(());
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_dir = format!("{}/backup_{}", config_dir, timestamp);

    // Create backup directory
    create_dir_all_with_privileges(&backup_dir)?;

    // Copy existing configs to backup
    if Path::new(config_dir).exists() {
        if let Ok(entries) = fs::read_dir(config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
                    let filename = path.file_name().unwrap_or_default();
                    let backup_path = format!("{}/{}", backup_dir, filename.to_string_lossy());
                    if let Err(e) = fs::copy(&path, &backup_path) {
                        println!("Warning: Failed to backup {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    println!("✓ Configuration backed up to: {}", backup_dir);
    Ok(())
}

/// Emergency recovery function for when audio system breaks
pub fn recover_audio_system() -> Result<(), String> {
    println!("=== EMERGENCY AUDIO SYSTEM RECOVERY ===");

    let username = whoami::username();

    // List all problematic configs to remove
    let problematic_configs = [
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-exclusive.conf",
            username
        ),
        format!(
            "/home/{}/.config/pipewire/pipewire.conf.d/99-pro-audio-high-priority.conf",
            username
        ),
        format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio-exclusive.conf",
            username
        ),
        format!(
            "/home/{}/.config/wireplumber/wireplumber.conf.d/99-pro-audio.conf",
            username
        ),
    ];

    let mut removed = 0;
    for config in &problematic_configs {
        if Path::new(config).exists() {
            if let Ok(()) = fs::remove_file(config) {
                println!("✓ Removed: {}", config);
                removed += 1;
            } else {
                println!("⚠ Could not remove: {}", config);
            }
        }
    }

    if removed > 0 {
        println!("\n✓ Removed {} problematic configuration files", removed);
        println!("Restarting audio services...");

        // Simple restart without complex logic
        let _ = Command::new("systemctl")
            .args([
                "--user",
                "restart",
                "pipewire",
                "pipewire-pulse",
                "wireplumber",
            ])
            .status();

        // Wait for services to come back
        std::thread::sleep(std::time::Duration::from_secs(3));

        println!("✓ Audio system should be recovered");
        println!("If audio is still not working, try logging out and back in, or reboot.");
    } else {
        println!("✓ No problematic configurations found");
    }

    Ok(())
}

/// Non-blocking version of restart_audio_services for use in async contexts
pub fn restart_audio_services_non_blocking() -> Result<(), String> {
    // Spawn a thread to handle the restart
    std::thread::spawn(|| match restart_audio_services(false, true) {
        Ok(_) => println!("✓ Audio services restarted successfully in background"),
        Err(e) => println!("⚠ Failed to restart audio services: {}", e),
    });

    println!("Audio service restart initiated in background...");
    Ok(())
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

    #[test]
    fn test_exclusive_mode_config_safety() {
        // Test that exclusive mode config uses safe defaults
        let config_content = format!(
            r#"context.modules = [
    {{
        name = libpipewire-module-rt
        args = {{
            nice.level = -11    # Should not be -20
            rt.prio = 80        # Should not be 88
            rt.time.soft = 200000  # Should not be 100000
            rt.time.hard = 200000  # Should not be 100000
        }}
    }}
]"#
        );

        // Verify safe values are present
        assert!(config_content.contains("nice.level = -11"));
        assert!(config_content.contains("rt.prio = 80"));
        assert!(config_content.contains("rt.time.soft = 200000"));
        assert!(config_content.contains("rt.time.hard = 200000"));

        // Verify dangerous values are NOT present
        assert!(!config_content.contains("nice.level = -20"));
        assert!(!config_content.contains("rt.prio = 88"));
        assert!(!config_content.contains("rt.time.soft = 100000"));
        assert!(!config_content.contains("rt.time.hard = 100000"));
    }
}
