/*
 * Pro Audio Config - Preferences Module
 * Version: 2.1
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Application preferences management
 * Handles loading and saving user configuration preferences
 */

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppPreferences {
    pub system_wide_config: bool,
}

impl AppPreferences {
    pub fn load() -> Self {
        if let Some(prefs_dir) =
            directories::ProjectDirs::from("com", "proaudioconfig", "Pro Audio Config")
        {
            let prefs_path = prefs_dir.config_dir().join("preferences.toml");
            if let Ok(content) = fs::read_to_string(&prefs_path)
                && let Ok(prefs) = toml::from_str(&content)
            {
                return prefs;
            }
        }
        AppPreferences::default()
    }

    pub fn save(prefs: &Self) -> Result<(), String> {
        if let Some(prefs_dir) =
            directories::ProjectDirs::from("com", "proaudioconfig", "Pro Audio Config")
        {
            let config_dir = prefs_dir.config_dir();
            println!("DEBUG: Saving preferences to: {}", config_dir.display());

            fs::create_dir_all(config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;

            let prefs_path = config_dir.join("preferences.toml");
            println!("DEBUG: Preferences file path: {}", prefs_path.display());

            let content = toml::to_string(prefs)
                .map_err(|e| format!("Failed to serialize preferences: {}", e))?;

            fs::write(&prefs_path, content)
                .map_err(|e| format!("Failed to write preferences: {}", e))?;

            println!(
                "DEBUG: Preferences saved successfully to: {}",
                prefs_path.display()
            );
        }
        Ok(())
    }
}
