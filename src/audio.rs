/*
 * Pro Audio Config - Audio Configuration Module
 * Version: 1.3
 * Copyright (c) 2025 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Core audio device detection and system interaction
 */

use std::process::Command;

#[derive(Clone, Debug)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub id: String,
    pub device_type: DeviceType,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq)]
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
    pub device_id: String,
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

    pub fn validate(&self) -> Result<(), String> {
        const VALID_SAMPLE_RATES: [u32; 5] = [44100, 48000, 96000, 192000, 384000];
        const VALID_BIT_DEPTHS: [u32; 3] = [16, 24, 32];
        const VALID_BUFFER_SIZES: [u32; 7] = [128, 256, 512, 1024, 2048, 4096, 8192];

        if !VALID_SAMPLE_RATES.contains(&self.sample_rate) {
            return Err(format!("Invalid sample rate: {}. Valid rates: {:?}", self.sample_rate, VALID_SAMPLE_RATES));
        }

        if !VALID_BIT_DEPTHS.contains(&self.bit_depth) {
            return Err(format!("Invalid bit depth: {}. Valid depths: {:?}", self.bit_depth, VALID_BIT_DEPTHS));
        }

        if !VALID_BUFFER_SIZES.contains(&self.buffer_size) {
            return Err(format!("Invalid buffer size: {}. Valid sizes: {:?}", self.buffer_size, VALID_BUFFER_SIZES));
        }

        if self.device_id.is_empty() {
            return Err("Device ID cannot be empty".to_string());
        }

        if !is_valid_device_id(&self.device_id) {
            return Err(format!("Invalid device ID format: {}. Expected: 'default', 'alsa:...', 'pipewire:...', 'pulse:...'", self.device_id));
        }

        Ok(())
    }

    pub fn get_audio_format(&self) -> Result<&'static str, String> {
        match self.bit_depth {
            16 => Ok("S16LE"),
            24 => Ok("S24LE"),
            32 => Ok("S32LE"),
            _ => Err(format!("Invalid bit depth: {}", self.bit_depth)),
        }
    }
}

fn is_valid_device_id(device_id: &str) -> bool {
    // Empty device ID is always invalid
    if device_id.is_empty() {
        return false;
    }
    
    // Check for specific valid patterns
    if device_id == "default" ||
       device_id.starts_with("alsa:") ||
       device_id.starts_with("pipewire:") ||
       device_id.starts_with("pulse:") {
        return true;
    }
    
    // For other device IDs, they must not contain whitespace
    !device_id.chars().any(|c| c.is_whitespace())
}

// Device Detection Functions
pub fn detect_all_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    println!("=== Scanning for all audio devices ===");
    
    if let Ok(output) = Command::new("pw-cli").args(["list-objects", "Node"]).output() {
        devices.extend(parse_pipewire_devices(&output.stdout)?);
    }
    
    devices.extend(detect_alsa_devices()?);
    devices.extend(detect_pulse_devices()?);
    
    println!("Found {} audio devices", devices.len());
    Ok(devices)
}

fn is_real_hardware_device(device: &AudioDevice) -> bool {
    let name = device.name.to_lowercase();
    let description = device.description.to_lowercase();
    
    // Skip virtual devices and internal nodes
    let virtual_indicators = ["virtual", "null", "dummy", "echo-cancel", "monitor", "proaudio"];
    for indicator in virtual_indicators {
        if name.contains(indicator) || description.contains(indicator) {
            return false;
        }
    }
    
    // Device-type specific filtering
    match device.id.split(':').next() {
        Some("pipewire") => {
            if description.contains("internal") && 
               !description.contains("usb") && 
               !description.contains("hdmi") && 
               !description.contains("analog") {
                return false;
            }
        }
        Some("alsa") => {
            let alsa_virtual = ["default", "dmix", "dsnoop", "hw", "plughw", "lavrate", 
                               "samplerate", "speexrate", "variable", "rate_convert", 
                               "linear", "mu-law", "a-law", "float", "oss", "pulse", 
                               "upmix", "vdownmix", "usbstream"];
            if alsa_virtual.iter().any(|&v| name.contains(v)) {
                return false;
            }
        }
        Some("pulse") => {
            if name.contains("module-") || name == "auto_null" {
                return false;
            }
        }
        _ => {}
    }
    
    true
}

fn parse_pipewire_devices(output: &[u8]) -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    let output_str = String::from_utf8_lossy(output);
    let mut current_device: Option<AudioDevice> = None;
    
    for line in output_str.lines() {
        if line.contains("object:") && line.contains("Node") {
            if let Some(device) = current_device.take() {
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
        }
        
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
            
            if line.contains("media.class") && line.contains('=') {
                if let Some(class) = line.split('=').nth(1) {
                    let class_clean = class.trim().trim_matches('"');
                    device.device_type = classify_device_type(class_clean, device);
                }
            }
        }
    }
    
    if let Some(device) = current_device.take() {
        if is_real_hardware_device(&device) {
            devices.push(device);
        }
    }
    
    Ok(devices)
}

fn classify_device_type(class: &str, device: &AudioDevice) -> DeviceType {
    match class {
        s if s.contains("Audio/Source") => DeviceType::Input,
        s if s.contains("Audio/Sink") => DeviceType::Output,
        s if s.contains("Audio/Duplex") => DeviceType::Duplex,
        s if s.contains("Audio") => {
            let name_lower = device.name.to_lowercase();
            let desc_lower = device.description.to_lowercase();
            if name_lower.contains("input") || desc_lower.contains("input") || desc_lower.contains("capture") {
                DeviceType::Input
            } else if name_lower.contains("output") || desc_lower.contains("output") || desc_lower.contains("playback") {
                DeviceType::Output
            } else {
                DeviceType::Unknown
            }
        }
        _ => DeviceType::Unknown,
    }
}

fn detect_alsa_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    if let Ok(output) = Command::new("aplay").args(["-L"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_alsa_output(&output_str, DeviceType::Output));
    }
    
    if let Ok(output) = Command::new("arecord").args(["-L"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_alsa_output(&output_str, DeviceType::Input));
    }
    
    Ok(devices)
}

fn parse_alsa_output(output: &str, device_type: DeviceType) -> Vec<AudioDevice> {
    output.lines()
        .filter(|line| !line.starts_with(' ') && !line.is_empty() && *line != "default")
        .filter_map(|line| {
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
            if is_real_hardware_device(&device) {
                Some(device)
            } else {
                None
            }
        })
        .collect()
}

fn detect_pulse_devices() -> Result<Vec<AudioDevice>, String> {
    let mut devices = Vec::new();
    
    if let Ok(output) = Command::new("pactl").args(["list", "sinks", "short"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_pulse_output(&output_str, DeviceType::Output));
    }
    
    if let Ok(output) = Command::new("pactl").args(["list", "sources", "short"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        devices.extend(parse_pulse_output(&output_str, DeviceType::Input));
    }
    
    Ok(devices)
}

fn parse_pulse_output(output: &str, device_type: DeviceType) -> Vec<AudioDevice> {
    output.lines()
        .filter_map(|line| {
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
                if is_real_hardware_device(&device) {
                    Some(device)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn extract_id(line: &str) -> String {
    line.split(',')
        .find(|s| s.trim().starts_with("id:"))
        .and_then(|id_part| id_part.split(':').nth(1))
        .map(|id| format!("pipewire:{}", id.trim()))
        .unwrap_or_else(|| "unknown".to_string())
}

// System Detection Functions
pub fn detect_audio_device() -> Result<String, String> {
    let audio_system = detect_audio_system();
    
    if let Ok(output) = Command::new("pactl").args(["info"]).output() {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("Default Sink:") {
                    let sink = line.replace("Default Sink:", "").trim().to_string();
                    return Ok(format!("{}: {}", audio_system, sink));
                }
            }
        }
    }
    
    Ok(format!("{}: Unknown Audio Device", audio_system))
}

pub fn detect_current_audio_settings() -> Result<AudioSettings, String> {
    if let Ok(output) = Command::new("pw-cli").args(["info", "0"]).output() {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let (sample_rate, bit_depth, buffer_size) = parse_pipewire_settings(&output_str);
            return Ok(AudioSettings::new(sample_rate, bit_depth, buffer_size, "default".to_string()));
        }
    }
    
    // Fallbacks
    if Command::new("pactl").args(["info"]).output().is_ok() {
        return Ok(AudioSettings::new(48000, 24, 512, "default".to_string()));
    }
    
    Ok(AudioSettings::new(48000, 24, 512, "default".to_string()))
}

fn parse_pipewire_settings(output: &str) -> (u32, u32, u32) {
    let mut sample_rate = 48000;
    let mut bit_depth = 24;
    let mut buffer_size = 512;
    
    for line in output.lines() {
        let trimmed = line.trim();
        
        // Parse sample rate - look for exact match
        if trimmed.starts_with("default.clock.rate =") && !trimmed.contains("allowed-rates") {
            if let Some(rate_str) = trimmed.split('=').nth(1) {
                let rate_clean = rate_str.trim().trim_matches('"').trim();
                if let Ok(rate) = rate_clean.parse::<u32>() {
                    sample_rate = rate;
                }
            }
        }
        
        // Parse audio format
        if trimmed.starts_with("audio.format =") {
            if let Some(format_str) = trimmed.split('=').nth(1) {
                let format = format_str.trim().trim_matches('"').trim();
                bit_depth = match format {
                    "S16LE" => 16,
                    "S24LE" => 24, 
                    "S32LE" => 32,
                    _ => 24,
                };
            }
        }
        
        // Parse buffer size - only the main quantum value
        if trimmed.starts_with("default.clock.quantum =") {
            if let Some(quantum_str) = trimmed.split('=').nth(1) {
                let quantum_clean = quantum_str.trim().trim_matches('"').trim();
                if let Ok(quantum) = quantum_clean.parse::<u32>() {
                    buffer_size = quantum;
                }
            }
        }
    }
    
    (sample_rate, bit_depth, buffer_size)
}

fn detect_audio_system() -> String {
    // Check PipeWire
    if Command::new("pw-cli").args(["info", "0"]).output().is_ok() ||
       Command::new("systemctl").args(["--user", "is-active", "pipewire"]).output().is_ok() {
        return "PipeWire".to_string();
    }
    
    // Check PulseAudio
    if Command::new("pulseaudio").args(["--check"]).output().is_ok() ||
       Command::new("systemctl").args(["--user", "is-active", "pulseaudio"]).output().is_ok() {
        return "PulseAudio".to_string();
    }
    
    "Audio System".to_string()
}

// Device Resolution Functions
pub fn resolve_pipewire_device_name(node_id: &str) -> Result<String, String> {
    let output = Command::new("pw-cli")
        .args(["info", node_id])
        .output()
        .map_err(|e| format!("Failed to query PipeWire node {}: {}", node_id, e))?;
    
    if !output.status.success() {
        return Err(format!("PipeWire query failed for node {}", node_id));
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if line.contains("node.name") && line.contains('=') {
            if let Some(name_part) = line.split('=').nth(1) {
                let name = name_part.trim().trim_matches('"').to_string();
                if !name.is_empty() {
                    return Ok(name);
                }
            }
        }
    }
    
    Err(format!("Could not find node.name for PipeWire node {}", node_id))
}

pub fn resolve_pulse_device_name(pulse_id: &str) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(["list", "sinks", "short"])
        .output()
        .map_err(|e| format!("Failed to query PulseAudio devices: {}", e))?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == pulse_id {
                return Ok(parts[1].to_string());
            }
        }
    }
    
    // Check sources as fallback
    if let Ok(output) = Command::new("pactl").args(["list", "sources", "short"]).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == pulse_id {
                return Ok(parts[1].to_string());
            }
        }
    }
    
    Err(format!("PulseAudio device {} not found", pulse_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_settings_validation() {
        let valid = AudioSettings::new(48000, 24, 512, "default".to_string());
        assert!(valid.validate().is_ok());

        let invalid_rate = AudioSettings::new(12345, 24, 512, "default".to_string());
        assert!(invalid_rate.validate().is_err());

        let invalid_depth = AudioSettings::new(48000, 8, 512, "default".to_string());
        assert!(invalid_depth.validate().is_err());
    }

    #[test]
    fn test_device_id_validation() {
        assert!(is_valid_device_id("default"));
        assert!(is_valid_device_id("alsa:device1"));
        assert!(is_valid_device_id("pipewire:123"));
        assert!(is_valid_device_id("pulse:0"));
        assert!(!is_valid_device_id(""));
        assert!(!is_valid_device_id("invalid device"));
    }

    #[test]
    fn test_audio_format_selection() {
        let settings_16 = AudioSettings::new(48000, 16, 512, "default".to_string());
        assert_eq!(settings_16.get_audio_format().unwrap(), "S16LE");

        let settings_24 = AudioSettings::new(48000, 24, 512, "default".to_string());
        assert_eq!(settings_24.get_audio_format().unwrap(), "S24LE");

        let settings_32 = AudioSettings::new(48000, 32, 512, "default".to_string());
        assert_eq!(settings_32.get_audio_format().unwrap(), "S32LE");

        let settings_invalid = AudioSettings::new(48000, 8, 512, "default".to_string());
        assert!(settings_invalid.get_audio_format().is_err());
    }

    #[test]
    fn test_device_type_classification() {
        let mut device = AudioDevice {
            name: "test".to_string(),
            description: "test".to_string(),
            id: "test".to_string(),
            device_type: DeviceType::Unknown,
            available: true,
        };

        assert!(matches!(classify_device_type("Audio/Source", &device), DeviceType::Input));
        assert!(matches!(classify_device_type("Audio/Sink", &device), DeviceType::Output));
        assert!(matches!(classify_device_type("Audio/Duplex", &device), DeviceType::Duplex));
        
        device.name = "input_device".to_string();
        assert!(matches!(classify_device_type("Audio", &device), DeviceType::Input));
        
        device.name = "output_device".to_string();
        assert!(matches!(classify_device_type("Audio", &device), DeviceType::Output));
    }
}
