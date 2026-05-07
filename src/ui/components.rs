/*
 * Pro Audio Config - UI Components Module
 * Version: 2.0
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 * Feel free to share and modify
 *
 * Reusable UI components, widgets, and helper functions
 * for creating consistent GTK3 interface elements
 */

use gtk::prelude::*;
use gtk::{Box as GtkBox, ComboBoxText, Frame, Label, Orientation, Separator};

/// Helper function to create constrained combo boxes that don't expand too wide
pub fn create_constrained_combo() -> ComboBoxText {
    let combo = ComboBoxText::new();
    combo.set_hexpand(false);
    combo.set_halign(gtk::Align::Fill);
    combo.set_width_request(400);
    combo
}

/// Helper function to populate combo boxes from common definitions
pub fn populate_combo_box(combo: &ComboBoxText, options: &[(u32, &str)]) {
    for (value, label) in options {
        combo.append(Some(&value.to_string()), label);
    }
}

pub fn create_section_box(title: &str) -> (Frame, GtkBox) {
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
