/*
 * Pro Audio Config - Dialogs Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 *
 * Dialog helper functions
 */

use gtk::prelude::*;
use gtk::{AboutDialog, ButtonsType, DialogFlags, MessageDialog, MessageType, Window};

pub fn show_error_dialog(message: &str) {
    let dialog = MessageDialog::new::<Window>(
        None,
        DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        "Failed to apply audio settings",
    );

    dialog.set_title("Error Applying Settings");

    let display_message = if message.contains("Script failed") {
        if let Some(error_part) = message.split("\n\n").nth(1) {
            error_part.to_string()
        } else {
            message.to_string()
        }
    } else {
        message.to_string()
    };

    dialog.set_secondary_text(Some(&display_message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show_all();
}

pub fn show_success_dialog(message: &str) {
    let dialog = MessageDialog::new::<Window>(
        None,
        DialogFlags::MODAL,
        MessageType::Info,
        ButtonsType::Ok,
        "Settings Applied Successfully",
    );

    dialog.set_secondary_text(Some(message));

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show_all();
}

pub fn show_about_dialog() {
    let dialog = AboutDialog::new();

    dialog.set_title("About Pro Audio Config");
    dialog.set_program_name("Pro Audio Config");
    dialog.set_version(Some("2.0"));
    dialog.set_website(Some("https://github.com/Peter-L-SVK/pro_audio_config"));
    dialog.set_copyright(Some("Copyright © 2025 Peter Leukanič"));
    dialog.set_authors(&["Peter Leukanič"]);
    dialog.set_comments(Some(
        "Professional audio configuration tool for Linux\nWritten in Rust programming language and Gtk-3.0",
    ));
    dialog.set_license_type(gtk::License::MitX11);

    dialog.set_license(Some(
        "This program is free software: you can redistribute it and/or modify\n\
         it under the terms of the MIT License.\n\
         \n\
         This program is distributed in the hope that it will be useful,\n\
         but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
         MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.\n\
         \n\
         For more details, see the LICENSE file included with this distribution.",
    ));
    dialog.set_wrap_license(true);

    // Try to set the application icon from multiple possible locations
    let icon_paths = [
        // System installation paths (multiple sizes)
        "/usr/share/icons/hicolor/16x16/apps/pro-audio-config.png",
        "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
        "/usr/share/icons/hicolor/32x32/apps/pro-audio-config.png",
        "/usr/share/icons/hicolor/256x256/apps/pro-audio-config.png",
        // Development paths
        "icons/48x48/pro-audio-config.png",
        "icons/32x32/pro-audio-config.png",
        "icons/icon.png",
        "icon.png",
        "../icons/icon.png",
        "./icons/icon.png",
        // Alternative system paths
        "/usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png",
    ];

    for path in &icon_paths {
        if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(path) {
            dialog.set_logo(Some(&pixbuf));
            break;
        }
    }

    dialog.set_modal(true);

    // Try to set transient parent to the main window
    if let Some(window) = get_main_window() {
        dialog.set_transient_for(Some(&window));
    }

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.show_all();
}

/// Helper function to get the main application window
fn get_main_window() -> Option<gtk::Window> {
    // Try to find the application's main window from toplevel widgets
    for widget in gtk::Window::list_toplevels() {
        if let Some(window) = widget.downcast_ref::<gtk::Window>()
            && window.is_visible()
        {
            return Some(window.clone());
        }
    }
    None
}
