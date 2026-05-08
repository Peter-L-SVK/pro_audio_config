/*
 * Pro Audio Config - Configuration Inspector Module
 * Version: 2.1
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 *
 * Configuration file inspection, management, and comparison
 */

use chrono::{DateTime, Local};
use glib::ControlFlow;
use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, CellRendererText, Frame, Label, ListStore, Orientation, ScrolledWindow,
    Separator, TreeView, TreeViewColumn, Window,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use whoami::username;

#[derive(Clone, Debug)]
pub struct ConfigFileInfo {
    pub path: PathBuf,
    pub filename: String,
    pub size: u64,
    pub modified: DateTime<Local>,
    pub owner: String,
    pub is_system: bool,
    pub is_active: bool,
    pub first_lines: String,
}

#[derive(Clone)]
pub struct ConfigInspectorTab {
    pub container: GtkBox,
    pub status_label: Label,
    pub refresh_button: Button,
    pub user_store: ListStore,
    pub system_store: ListStore,
    scan_in_progress: Arc<Mutex<bool>>,
}

impl Default for ConfigInspectorTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigInspectorTab {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);

        // ===== STATUS SECTION =====
        let (status_frame, status_box) = create_section_box("Configuration Inspector Status");

        let status_label = Label::new(Some("Ready to scan configuration files..."));
        status_label.set_halign(gtk::Align::Start);

        let refresh_button = Button::with_label("Refresh Configuration Scan");
        refresh_button
            .set_tooltip_text(Some("Rescan configuration files and active PipeWire state"));

        let info_label = Label::new(Some(
            "This tab shows all PipeWire/WirePlumber configuration files and their current status.",
        ));
        info_label.set_line_wrap(true);
        info_label.set_halign(gtk::Align::Start);

        status_box.pack_start(&status_label, false, false, 0);
        status_box.pack_start(&refresh_button, false, false, 6);
        status_box.pack_start(&info_label, false, false, 0);

        // ===== USER CONFIGS SECTION =====
        let (user_frame, user_box) = create_section_box("User Configuration Files");

        let user_info_label = Label::new(Some(
            "User configuration files (~/.config/pipewire/ and ~/.config/wireplumber/)",
        ));
        user_info_label.set_line_wrap(true);
        user_info_label.set_halign(gtk::Align::Start);

        let user_scrolled = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        user_scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        user_scrolled.set_min_content_height(200);

        let (user_tree, user_store) = Self::create_config_tree_view();
        user_scrolled.add(&user_tree);

        user_box.pack_start(&user_info_label, false, false, 0);
        user_box.pack_start(&user_scrolled, true, true, 0);

        // ===== SYSTEM CONFIGS SECTION =====
        let (system_frame, system_box) = create_section_box("System Configuration Files");

        let system_info_label = Label::new(Some(
            "System configuration files (/etc/pipewire/ and /etc/wireplumber/)",
        ));
        system_info_label.set_line_wrap(true);
        system_info_label.set_halign(gtk::Align::Start);

        let system_scrolled =
            ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        system_scrolled.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
        system_scrolled.set_min_content_height(200);

        let (system_tree, system_store) = Self::create_config_tree_view();
        system_scrolled.add(&system_tree);

        system_box.pack_start(&system_info_label, false, false, 0);
        system_box.pack_start(&system_scrolled, true, true, 0);

        // ===== ACTIONS SECTION =====
        let (actions_frame, actions_box) = create_section_box("File Actions");

        let actions_info_label = Label::new(Some(
            "Double-click on any file to open it with the default system editor.",
        ));
        actions_info_label.set_line_wrap(true);
        actions_info_label.set_halign(gtk::Align::Start);

        let legend_box = GtkBox::new(Orientation::Horizontal, 12);
        legend_box.set_halign(gtk::Align::Start);

        let active_label = Label::new(Some("✓ = Currently active in PipeWire"));
        let inactive_label = Label::new(Some("  = Not active"));

        legend_box.pack_start(&active_label, false, false, 0);
        legend_box.pack_start(&inactive_label, false, false, 0);

        actions_box.pack_start(&actions_info_label, false, false, 0);
        actions_box.pack_start(&legend_box, false, false, 6);

        // ===== ASSEMBLE TAB =====
        container.pack_start(&status_frame, false, false, 0);
        container.pack_start(&user_frame, true, true, 0);
        container.pack_start(&system_frame, true, true, 0);
        container.pack_start(&actions_frame, false, false, 0);

        let tab = ConfigInspectorTab {
            container,
            status_label,
            refresh_button,
            user_store,
            system_store,
            scan_in_progress: Arc::new(Mutex::new(false)),
        };

        // Set up double-click events
        tab.setup_double_click_events(&user_tree, false);
        tab.setup_double_click_events(&system_tree, true);

        // Connect refresh button
        let tab_clone = tab.clone();
        tab.refresh_button.connect_clicked(move |_| {
            tab_clone.scan_configs();
        });

        // Trigger initial scan
        let tab_for_timeout = tab.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            tab_for_timeout.scan_configs();
            ControlFlow::Break
        });

        tab
    }

    fn create_config_tree_view() -> (TreeView, ListStore) {
        let store = ListStore::new(&[
            gtk::glib::Type::STRING, // Status indicator
            gtk::glib::Type::STRING, // File name
            gtk::glib::Type::STRING, // Modified time
            gtk::glib::Type::STRING, // Size
        ]);

        let tree_view = TreeView::with_model(&store);

        // File name column with status indicator
        let name_col = TreeViewColumn::new();
        let name_cell = CellRendererText::new();
        gtk::prelude::CellLayoutExt::pack_start(&name_col, &name_cell, true);
        gtk::prelude::CellLayoutExt::add_attribute(&name_col, &name_cell, "text", 0);
        name_col.set_title("File");
        name_col.set_resizable(true);
        name_col.set_min_width(200);
        tree_view.append_column(&name_col);

        // Modified column
        let mod_col = TreeViewColumn::new();
        let mod_cell = CellRendererText::new();
        gtk::prelude::CellLayoutExt::pack_start(&mod_col, &mod_cell, false);
        gtk::prelude::CellLayoutExt::add_attribute(&mod_col, &mod_cell, "text", 2);
        mod_col.set_title("Modified");
        mod_col.set_resizable(true);
        mod_col.set_min_width(120);
        tree_view.append_column(&mod_col);

        // Size column
        let size_col = TreeViewColumn::new();
        let size_cell = CellRendererText::new();
        gtk::prelude::CellLayoutExt::pack_start(&size_col, &size_cell, false);
        gtk::prelude::CellLayoutExt::add_attribute(&size_col, &size_cell, "text", 3);
        size_col.set_title("Size");
        size_col.set_resizable(true);
        size_col.set_min_width(80);
        tree_view.append_column(&size_col);

        tree_view.set_headers_clickable(true);

        (tree_view, store)
    }

    fn setup_double_click_events(&self, tree_view: &TreeView, is_system: bool) {
        let store = if is_system {
            self.system_store.clone()
        } else {
            self.user_store.clone()
        };

        let tree_view_clone = tree_view.clone();
        tree_view.connect_row_activated(move |_, path, _| {
            if let Some(iter) = store.iter(path) {
                let filename: String = store.value(&iter, 1).get().unwrap();
                let full_path: String = filename;

                // Open the file
                Self::open_config_file(&full_path, is_system);
            }
        });
    }

    fn open_config_file(path: &str, is_system: bool) {
        if is_system {
            let path_clone = path.to_string();
            println!("Opening system file: {}", path);

            // Create a simple script that will ask for sudo and open editor
            let script = format!(
                r#"#!/bin/bash
# GNOME-based terminals
if [ -n "$GNOME_TERMINAL_SCREEN" ] || [ -n "$GNOME_TERMINAL_SERVICE" ]; then
    # GNOME Terminal
    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal -- bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# KDE Plasma - Konsole
elif [ -n "$KONSOLE_DBUS_SESSION" ] || [ "$XDG_CURRENT_DESKTOP" = "KDE" ] || [ "$DESKTOP_SESSION" = "plasma" ]; then
    if command -v konsole &> /dev/null; then
        konsole -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# MATE Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "MATE" ] || [ "$DESKTOP_SESSION" = "mate" ]; then
    if command -v mate-terminal &> /dev/null; then
        mate-terminal -- bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Cinnamon Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "X-Cinnamon" ] || [ "$DESKTOP_SESSION" = "cinnamon" ]; then
    if command -v gnome-terminal &> /dev/null; then
        # Cinnamon often uses gnome-terminal
        gnome-terminal -- bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    elif command -v x-terminal-emulator &> /dev/null; then
        x-terminal-emulator -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# LXQt Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "LXQt" ] || [ "$DESKTOP_SESSION" = "lxqt" ]; then
    if command -v qterminal &> /dev/null; then
        qterminal -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# LXDE Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "LXDE" ] || [ "$DESKTOP_SESSION" = "LXDE" ]; then
    if command -v lxterminal &> /dev/null; then
        lxterminal -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Xfce Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "XFCE" ] || [ "$DESKTOP_SESSION" = "xfce" ]; then
    if command -v xfce4-terminal &> /dev/null; then
        xfce4-terminal -x bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# COSMIC Desktop (System76)
elif [ "$XDG_CURRENT_DESKTOP" = "COSMIC" ] || echo "$DESKTOP_SESSION" | grep -qi cosmic; then
    # COSMIC typically uses GNOME infrastructure
    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal -- bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Budgie Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "Budgie:GNOME" ] || [ "$DESKTOP_SESSION" = "budgie-desktop" ]; then
    if command -v gnome-terminal &> /dev/null; then
        gnome-terminal -- bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Pantheon (elementary OS)
elif [ "$XDG_CURRENT_DESKTOP" = "Pantheon" ]; then
    if command -v io.elementary.terminal &> /dev/null; then
        io.elementary.terminal -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    elif command -v pantheon-terminal &> /dev/null; then
        pantheon-terminal -e "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Deepin Desktop
elif [ "$XDG_CURRENT_DESKTOP" = "Deepin" ]; then
    if command -v deepin-terminal &> /dev/null; then
        deepin-terminal -e "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi

# Enlightenment
elif [ "$XDG_CURRENT_DESKTOP" = "Enlightenment" ] || [ "$DESKTOP_SESSION" = "enlightenment" ]; then
    if command -v terminology &> /dev/null; then
        terminology -e "sudoedit {}; echo 'Press Enter to close...'; read"
        exit 0
    fi
fi

# Generic terminal detection as fallback
# Check for common terminals regardless of DE
for terminal_cmd in \
    "$TERMINAL" \
    "x-terminal-emulator" \
    "urxvt" \
    "rxvt" \
    "st" \
    "alacritty" \
    "kitty" \
    "terminator" \
    "tilix" \
    "termite" \
    "sakura" \
    "terminology" \
    "roxterm" \
    "cool-retro-term" \
    "hyper" \
    "wezterm" \
    "foot" \
    "xterm"
do
    if command -v "$terminal_cmd" &> /dev/null; then
        case "$terminal_cmd" in
            "xterm"|"urxvt"|"rxvt"|"st")
                $terminal_cmd -e "sudoedit {}; echo 'Press Enter to close...'; read"
                ;;
            "alacritty"|"kitty"|"wezterm"|"foot")
                $terminal_cmd -e bash -c "sudoedit {}; echo 'Press Enter to close...'; read"
                ;;
            "terminator"|"tilix")
                $terminal_cmd -e "bash -c 'sudoedit {}; echo \"Press Enter to close...\"; read'"
                ;;
            *)
                $terminal_cmd -e "sudoedit {}; echo 'Press Enter to close...'; read"
                ;;
        esac
        exit 0
    fi
done"#,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path,
                path
            );

            let temp_script = format!("/tmp/proaudio_edit_{}.sh", std::process::id());

            if std::fs::write(&temp_script, &script).is_ok() {
                let _ = Command::new("chmod").args(["+x", &temp_script]).status();

                // Try to execute the script
                match Command::new("sh").arg(&temp_script).spawn() {
                    Ok(_) => {
                        println!("Opened terminal for editing");
                        show_success_dialog(&format!(
                            "Opened terminal for editing system file:\n{}",
                            path
                        ));
                    }
                    Err(e) => {
                        println!("Failed to open terminal: {}", e);
                        Self::show_manual_instructions(path);
                        show_error_dialog(&format!(
                            "Failed to open terminal for editing:\n{}\n\nSee terminal for manual instructions.",
                            e
                        ));
                    }
                }

                // Clean up after a delay
                let temp_script_clone = temp_script.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let _ = std::fs::remove_file(&temp_script_clone);
                });
            } else {
                Self::show_manual_instructions(path);
                show_error_dialog("Failed to create temporary script for opening terminal.");
            }
        } else {
            // User files
            let path_clone = path.to_string();
            match Command::new("xdg-open").arg(&path_clone).spawn() {
                Ok(_) => {
                    show_success_dialog(&format!("Opened user configuration file:\n{}", path));
                }
                Err(e) => {
                    show_error_dialog(&format!(
                        "Failed to open file with default editor:\n{}\n\nYou can manually open: {}",
                        e, path
                    ));
                }
            }
        }
    }

    fn show_manual_instructions(path: &str) {
        let instructions = format!(
            "SYSTEM FILE EDITING INSTRUCTIONS\n\n\
            File: {}\n\n\
            To edit this system configuration file:\n\n\
            OPTION 1 - Recommended:\n\
            • Open a terminal and run:\n\
              sudoedit {}\n\n\
            OPTION 2 - Alternative:\n\
            • Open a terminal and run:\n\
              sudo nano {}\n",
            path, path, path
        );

        show_error_dialog(&instructions);

        println!("\n═══════════════════════════════════════════════════════════");
        println!("  SYSTEM FILE EDITING INSTRUCTIONS");
        println!("═══════════════════════════════════════════════════════════");
        println!("File: {}", path);
        println!();
        println!("To edit this system configuration file:");
        println!();
        println!("OPTION 1 - Recommended:");
        println!("  Open a terminal and run:");
        println!("    sudoedit {}", path);
        println!();
        println!("OPTION 2 - Alternative:");
        println!("  Open a terminal and run:");
        println!("    sudo nano {}", path);
        println!();
        println!("OPTION 3 - Graphical editor (if available):");
        println!("  Open a terminal and run:");
        println!("    sudo gedit {}", path);
        println!("    (replace 'gedit' with your preferred editor)");
        println!("═══════════════════════════════════════════════════════════\n");
    }

    pub fn scan_configs(&self) {
        // Prevent duplicate scans
        {
            let mut in_progress = self.scan_in_progress.lock().unwrap();
            if *in_progress {
                println!("DEBUG: Scan already in progress, skipping...");
                return;
            }
            *in_progress = true;
        }
        let status_label = self.status_label.clone();
        let user_store = self.user_store.clone();
        let system_store = self.system_store.clone();

        status_label.set_text("Scanning configuration files...");

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            // Get running config first to determine active files
            let active_properties = match Self::get_active_config_properties() {
                Ok(props) => props,
                Err(e) => {
                    println!("Warning: Could not get active properties: {}", e);
                    HashMap::new()
                }
            };

            // Scan BOTH user and system configs in the same thread
            let (user_configs, _user_errors) =
                Self::scan_config_directory_with_errors(false, &active_properties);

            let (system_configs, _system_errors) =
                Self::scan_config_directory_with_errors(true, &active_properties);

            let user_len = user_configs.len();
            let system_len = system_configs.len();

            // Send all results at once
            let _ = tx.send((user_configs, system_configs, user_len, system_len));
        });

        let rx_arc = Arc::new(Mutex::new(rx));
        let rx_timeout: Arc<
            Mutex<mpsc::Receiver<(Vec<ConfigFileInfo>, Vec<ConfigFileInfo>, usize, usize)>>,
        > = Arc::clone(&rx_arc);

        glib::timeout_add_local(Duration::from_millis(100), move || {
            let rx_guard = rx_timeout.lock().unwrap();
            match rx_guard.try_recv() {
                Ok((user_configs, system_configs, user_len, system_len)) => {
                    // Clear and update user store
                    user_store.clear();
                    for config in &user_configs {
                        ConfigInspectorTab::add_config_to_store(&user_store, config);
                    }

                    // Clear and update system store
                    system_store.clear();
                    for config in &system_configs {
                        ConfigInspectorTab::add_config_to_store(&system_store, config);
                    }

                    let total = user_len + system_len;
                    let status_text = format!(
                        "Scan complete: {} user configs, {} system configs ({} total)",
                        user_len, system_len, total
                    );
                    status_label.set_text(&status_text);

                    // Show only ONE dialog with combined results
                    if total > 0 {
                        show_success_dialog(&format!(
                            "Configuration scan complete!\n\nFound {} configuration files:\n• {} user files\n• {} system files\n\nDouble-click any file to open it.",
                            total, user_len, system_len
                        ));
                    } else {
                        // Just update status, no error dialog needed
                        status_label.set_text("No configuration files found. This may be normal if PipeWire/WirePlumber use default settings.");
                    }

                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(_) => {
                    status_label.set_text("Scan interrupted");
                    ControlFlow::Break
                }
            }
        });
    }

    fn scan_config_directory_with_errors(
        is_system: bool,
        active_properties: &HashMap<String, Vec<String>>,
    ) -> (Vec<ConfigFileInfo>, Vec<String>) {
        let mut configs = Vec::new();
        let mut error_messages = Vec::new();

        let username = username();
        let home_path = format!("/home/{}", username);
        let base_path = if is_system {
            Path::new("/etc")
        } else {
            Path::new(&home_path)
        };

        // Define all directories to scan
        let scan_dirs: Vec<(PathBuf, &str)> = if is_system {
            vec![
                (PathBuf::from("/etc/pipewire"), "System PipeWire"),
                (
                    PathBuf::from("/etc/pipewire/pipewire.conf.d"),
                    "System PipeWire Config",
                ),
                (PathBuf::from("/etc/wireplumber"), "System WirePlumber"),
                (
                    PathBuf::from("/etc/wireplumber/wireplumber.conf.d"),
                    "System WirePlumber Config",
                ),
                (
                    PathBuf::from("/etc/wireplumber/main.lua.d"),
                    "System WirePlumber Lua",
                ),
                (
                    PathBuf::from("/usr/share/pipewire"),
                    "System PipeWire Defaults",
                ),
                (
                    PathBuf::from("/usr/share/wireplumber"),
                    "System WirePlumber Defaults",
                ),
            ]
        } else {
            vec![
                // PipeWire directories
                (
                    PathBuf::from(format!("{}/.config/pipewire", home_path)),
                    "User PipeWire",
                ),
                (
                    PathBuf::from(format!("{}/.config/pipewire/pipewire.conf.d", home_path)),
                    "User PipeWire Config",
                ),
                // WirePlumber directories
                (
                    PathBuf::from(format!("{}/.config/wireplumber", home_path)),
                    "User WirePlumber",
                ),
                (
                    PathBuf::from(format!(
                        "{}/.config/wireplumber/wireplumber.conf.d",
                        home_path
                    )),
                    "User WirePlumber Config",
                ),
                (
                    PathBuf::from(format!("{}/.config/wireplumber/main.lua.d", home_path)),
                    "User WirePlumber Lua",
                ),
                // Alternative locations
                (
                    PathBuf::from(format!("{}/.local/share/wireplumber", home_path)),
                    "User WirePlumber Local",
                ),
                (
                    PathBuf::from(format!(
                        "{}/.local/share/wireplumber/wireplumber.conf.d",
                        home_path
                    )),
                    "User WirePlumber Local Config",
                ),
            ]
        };

        for (dir_path, dir_label) in &scan_dirs {
            if !dir_path.exists() {
                println!(
                    "DEBUG: Directory does not exist: {} ({})",
                    dir_path.display(),
                    dir_label
                );
                continue;
            }

            println!("DEBUG: Scanning: {} ({})", dir_path.display(), dir_label);

            match fs::read_dir(dir_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        match Self::process_config_entry(&entry, is_system, active_properties) {
                            Ok(Some(info)) => {
                                println!("DEBUG: Found config: {}", info.filename);
                                configs.push(info);
                            }
                            Ok(None) => {} // Not a config file
                            Err(e) => error_messages.push(format!(
                                "Error processing {}: {}",
                                entry.path().display(),
                                e
                            )),
                        }
                    }
                }
                Err(e) => {
                    // Directory not readable is NOT an error for the user, just skip it
                    println!("DEBUG: Cannot read directory {}: {}", dir_path.display(), e);
                }
            }
        }

        // Also scan for main config files in the root config directories
        let main_conf_files: Vec<PathBuf> = if is_system {
            vec![
                PathBuf::from("/etc/pipewire/pipewire.conf"),
                PathBuf::from("/etc/wireplumber/wireplumber.conf"),
            ]
        } else {
            vec![
                PathBuf::from(format!("{}/.config/pipewire/pipewire.conf", home_path)),
                PathBuf::from(format!(
                    "{}/.config/wireplumber/wireplumber.conf",
                    home_path
                )),
            ]
        };

        for conf_path in &main_conf_files {
            if conf_path.exists() {
                println!("DEBUG: Found main config file: {}", conf_path.display());
                if let Ok(metadata) = fs::metadata(conf_path)
                    && metadata.is_file()
                {
                    match Self::get_file_info(conf_path, is_system, active_properties) {
                        Ok(info) => configs.push(info),
                        Err(e) => error_messages.push(format!(
                            "Error getting info for {}: {}",
                            conf_path.display(),
                            e
                        )),
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        configs.sort_by(|a, b| b.modified.cmp(&a.modified));

        // Deduplicate by full path
        let mut seen = HashSet::new();
        configs.retain(|config| {
            let key = config.path.to_string_lossy().to_string();
            seen.insert(key)
        });

        println!(
            "DEBUG: Found {} config files in {} mode",
            configs.len(),
            if is_system { "system" } else { "user" }
        );

        (configs, error_messages)
    }

    fn process_config_entry(
        entry: &fs::DirEntry,
        is_system: bool,
        active_properties: &HashMap<String, Vec<String>>,
    ) -> Result<Option<ConfigFileInfo>, String> {
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Cannot get file type for {:?}: {}", entry.path(), e))?;

        if !file_type.is_file() {
            return Ok(None);
        }

        let filename = entry.file_name();
        let filename_str = filename.to_string_lossy();

        // Check if it's a config file - expanded matching
        let is_config = filename_str.ends_with(".conf")
            || filename_str.ends_with(".lua")
            || filename_str.ends_with(".json")
            || filename_str.ends_with(".toml")
            || filename_str == "pipewire.conf"
            || filename_str == "wireplumber.conf"
            || filename_str.contains("pro-audio")
            || filename_str.contains("quantum");

        if !is_config {
            return Ok(None);
        }

        match Self::get_file_info(&entry.path(), is_system, active_properties) {
            Ok(info) => Ok(Some(info)),
            Err(e) => Err(e),
        }
    }

    fn get_file_info(
        path: &Path,
        is_system: bool,
        active_properties: &HashMap<String, Vec<String>>,
    ) -> Result<ConfigFileInfo, String> {
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Failed to get metadata for {:?}: {}", path, e))?;

        let modified: DateTime<Local> = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .into();

        let owner = if let Ok(output) = Command::new("stat")
            .args(["-c", "%U", path.to_str().unwrap()])
            .output()
        {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            "Unknown".to_string()
        };

        let content = fs::read_to_string(path).unwrap_or_else(|_| "Cannot read file".to_string());

        let first_lines = content
            .lines()
            .take(3)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<&str>>()
            .join(" | ");

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_active = Self::is_config_file_active(path)
            || Self::is_config_active_in_properties(&filename, active_properties);

        Ok(ConfigFileInfo {
            path: path.to_path_buf(),
            filename,
            size: metadata.len(),
            modified,
            owner,
            is_system,
            is_active,
            first_lines: if first_lines.len() > 50 {
                format!("{}...", &first_lines[..50])
            } else {
                first_lines
            },
        })
    }

    fn is_config_file_active(path: &Path) -> bool {
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if filename.starts_with("99-") || filename.contains("pro-audio") {
            return true;
        }

        if let Ok(content) = fs::read_to_string(path) {
            if content.contains("# Generated by Pro Audio Config") {
                return true;
            }

            let pro_audio_indicators = [
                "api.alsa.period-size",
                "api.alsa.headroom",
                "node.latency",
                "clock.rate",
                "audio.format",
                "audio.rate",
                "session.suspend-timeout-seconds",
                "priority.driver",
                "rt.prio",
                "mem.lock",
                "resample.quality",
            ];

            for indicator in &pro_audio_indicators {
                if content.contains(indicator) {
                    return true;
                }
            }
        }

        false
    }

    fn is_config_active_in_properties(
        filename: &str,
        active_properties: &HashMap<String, Vec<String>>,
    ) -> bool {
        for files in active_properties.values() {
            for file in files {
                if file == filename || file.contains(filename) || filename.contains(file) {
                    return true;
                }
            }
        }
        false
    }

    fn add_config_to_store(store: &ListStore, config: &ConfigFileInfo) {
        let status_indicator = if config.is_active { "✓ " } else { "  " };

        let display_name = format!("{}{}", status_indicator, config.filename);
        let modified_str = config.modified.format("%Y-%m-%d %H:%M").to_string();
        let size_str = if config.size > 1024 {
            format!("{:.1} KB", config.size as f64 / 1024.0)
        } else {
            format!("{} B", config.size)
        };

        let iter = store.append();
        store.set(
            &iter,
            &[
                (0, &display_name),
                (1, &config.path.to_string_lossy().to_string()),
                (2, &modified_str),
                (3, &size_str),
            ],
        );
    }

    fn get_active_config_properties() -> Result<HashMap<String, Vec<String>>, String> {
        let mut properties = HashMap::new();

        match Command::new("pw-dump").output() {
            Ok(output) => {
                if !output.status.success() {
                    return Ok(properties); // Not an error, just no data
                }

                match String::from_utf8(output.stdout) {
                    Ok(json_str) => match serde_json::from_str::<Value>(&json_str) {
                        Ok(parsed) => {
                            if let Some(array) = parsed.as_array() {
                                for item in array {
                                    if let Some(props) =
                                        item.get("info").and_then(|i| i.get("props"))
                                    {
                                        Self::extract_properties(props, &mut properties);
                                    }
                                }
                            }
                            Ok(properties)
                        }
                        Err(e) => {
                            println!("DEBUG: Failed to parse pw-dump JSON: {}", e);
                            Ok(properties)
                        }
                    },
                    Err(e) => {
                        println!("DEBUG: Failed to parse pw-dump output: {}", e);
                        Ok(properties)
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: pw-dump not available: {}", e);
                Ok(properties)
            }
        }
    }

    fn extract_properties(value: &Value, properties: &mut HashMap<String, Vec<String>>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    if key.contains("pro-audio") || key.contains("ProAudio") || key.contains("99-")
                    {
                        let files = properties.entry(key.clone()).or_default();
                        if let Some(filename) = Self::extract_filename_from_value(val)
                            && !files.contains(&filename)
                        {
                            files.push(filename);
                        }
                    } else if val.is_object() {
                        Self::extract_properties(val, properties);
                    } else if val.is_array() {
                        for item in val.as_array().unwrap() {
                            Self::extract_properties(item, properties);
                        }
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    Self::extract_properties(item, properties);
                }
            }
            _ => {}
        }
    }

    fn extract_filename_from_value(value: &Value) -> Option<String> {
        match value {
            Value::String(s) => {
                if s.contains(".conf") {
                    if let Some(start) = s.rfind('/') {
                        Some(s[start + 1..].to_string())
                    } else {
                        Some(s.clone())
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// Helper functions
fn create_section_box(title: &str) -> (Frame, GtkBox) {
    let frame = Frame::new(None);
    frame.set_margin_top(6);
    frame.set_margin_bottom(6);

    let section_box = GtkBox::new(Orientation::Vertical, 12);
    section_box.set_margin_top(12);
    section_box.set_margin_bottom(12);
    section_box.set_margin_start(12);
    section_box.set_margin_end(12);

    let title_label = Label::new(None);
    title_label.set_markup(&format!("<b>{}</b>", title));
    title_label.set_halign(gtk::Align::Start);

    section_box.pack_start(&title_label, false, false, 0);

    let separator = Separator::new(Orientation::Horizontal);
    section_box.pack_start(&separator, false, false, 0);

    frame.add(&section_box);

    (frame, section_box)
}

fn show_error_dialog(message: &str) {
    let dialog = gtk::MessageDialog::new(
        None::<&Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        "Error",
    );

    dialog.set_secondary_text(Some(message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show_all();
}

fn show_success_dialog(message: &str) {
    let dialog = gtk::MessageDialog::new(
        None::<&Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::Ok,
        "Success",
    );

    dialog.set_secondary_text(Some(message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file_info_creation() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test.conf");

        std::fs::write(&temp_file, "# Test config\npro-audio.test = true").unwrap();

        let active_props = HashMap::new();
        let info = ConfigInspectorTab::get_file_info(&temp_file, false, &active_props);

        assert!(info.is_ok());
        let info = info.unwrap();
        assert_eq!(info.filename, "test.conf");
        assert!(!info.is_system);

        std::fs::remove_file(temp_file).ok();
    }
}
