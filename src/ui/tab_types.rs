/*
 * Pro Audio Config - Tab Types Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Tab type definitions for Output and Input configuration tabs
 * Provides type-safe differentiation between input and output audio settings
 */

use crate::audio::AudioSettings;
use crate::audio::{
    AudioDevice, detect_input_audio_device, detect_input_audio_devices, detect_output_audio_device,
    detect_output_audio_devices,
};
use crate::config::{
    apply_input_audio_settings_with_auth_blocking, apply_output_audio_settings_with_auth_blocking,
};

#[derive(Clone, Debug)]
pub enum TabType {
    Output,
    Input,
}

impl TabType {
    pub fn title(&self) -> &'static str {
        match self {
            TabType::Output => "Output",
            TabType::Input => "Input",
        }
    }

    pub fn device_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Audio Device",
            TabType::Input => "Input Audio Device",
        }
    }

    pub fn settings_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Audio Settings",
            TabType::Input => "Input Audio Settings",
        }
    }

    pub fn actions_label(&self) -> &'static str {
        match self {
            TabType::Output => "Output Actions",
            TabType::Input => "Input Actions",
        }
    }

    pub fn apply_button_label(&self) -> &'static str {
        match self {
            TabType::Output => "Apply Output Audio Settings",
            TabType::Input => "Apply Input Audio Settings",
        }
    }

    pub fn detect_current_device_fn(&self) -> fn() -> Result<String, String> {
        match self {
            TabType::Output => detect_output_audio_device,
            TabType::Input => detect_input_audio_device,
        }
    }

    pub fn detect_devices_fn(&self) -> fn() -> Result<Vec<AudioDevice>, String> {
        match self {
            TabType::Output => detect_output_audio_devices,
            TabType::Input => detect_input_audio_devices,
        }
    }

    pub fn apply_settings_fn(&self) -> fn(AudioSettings) -> Result<(), String> {
        match self {
            TabType::Output => apply_output_audio_settings_with_auth_blocking,
            TabType::Input => apply_input_audio_settings_with_auth_blocking,
        }
    }

    pub fn selection_prefix(&self) -> &'static str {
        match self {
            TabType::Output => "Selected Output",
            TabType::Input => "Selected Input",
        }
    }

    pub fn current_device_prefix(&self) -> &'static str {
        match self {
            TabType::Output => "Current Default Output Device",
            TabType::Input => "Current Default Input Device",
        }
    }
}
