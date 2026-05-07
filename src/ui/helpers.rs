/*
 * Pro Audio Config - UI Helpers Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Utility helper functions for cleaning and formatting
 * device descriptions, display text, and device names
 */

/// Helper function to clean device description by removing status words like "SUSPENDED"
pub fn clean_device_description(description: &str) -> String {
    description
        .replace("SUSPENDED", "")
        .replace("RUNNING", "")
        .replace("IDLE", "")
        .trim()
        .trim_end_matches('-')
        .trim()
        .to_string()
}

/// Helper function to clean display text in the selection info
pub fn clean_display_text(display_text: &str) -> String {
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
pub fn clean_device_display(device_info: &str) -> String {
    device_info
        .replace("PipeWire:", "")
        .replace("PulseAudio:", "")
        .replace("ALSA:", "")
        .trim()
        .to_string()
}
