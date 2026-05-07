/*
 * Pro Audio Config - UI Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * User interface module declarations and re-exports
 * Organizes all UI submodules for the application
 */

pub mod advanced_tab;
pub mod app;
pub mod audio_tab;
pub mod components;
pub mod constants;
pub mod dialogs;
pub mod helpers;
pub mod preferences;
pub mod tab_types;

// Re-export main types for backward compatibility
pub use advanced_tab::AdvancedTab;
pub use app::AudioApp;
pub use audio_tab::AudioTab;
pub use components::create_section_box;
pub use constants::*;
pub use dialogs::{show_about_dialog, show_error_dialog, show_success_dialog};
pub use helpers::{clean_device_description, clean_device_display, clean_display_text};
pub use preferences::AppPreferences;
pub use tab_types::TabType;
