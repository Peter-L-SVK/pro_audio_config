/*
 * Pro Audio Config - AudioApp Module
 * Version: 2.1
 * Copyright (c) 2025-2026 Peter Leukanič
 * Under MIT License
 *
 * Main application window setup and tab management
 */

use gtk::prelude::*;
use gtk::{
    Adjustment, Application, ApplicationWindow, Box as GtkBox, Label, Menu, MenuBar, MenuItem,
    Notebook, Orientation, ScrolledWindow,
};

use super::advanced_tab::AdvancedTab;
use super::audio_tab::AudioTab;
use super::dialogs::show_about_dialog;
use super::tab_types::TabType;
use crate::config_inspector::ConfigInspectorTab;
use crate::monitoring::MonitoringTab;

#[derive(Clone)]
pub struct AudioApp {
    pub window: ApplicationWindow,
    pub notebook: Notebook,
    pub output_tab: AudioTab,
    pub input_tab: AudioTab,
    pub advanced_tab: AdvancedTab,
    pub monitoring_tab: MonitoringTab,
    pub config_inspector_tab: ConfigInspectorTab,
}

impl AudioApp {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);
        window.set_title("Pro Audio Config");
        window.set_default_size(900, 700);

        // Set window icon
        Self::set_window_icon(&window);

        // ===== MAIN VERTICAL BOX - Stacks menu bar on top of content =====
        let main_vbox = GtkBox::new(Orientation::Vertical, 0);

        // ===== MENU BAR - Fixed at the very top =====
        let menu_bar = MenuBar::new();

        let help_menu = Menu::new();
        let help_menu_item = MenuItem::with_label("Help");
        help_menu_item.set_submenu(Some(&help_menu));

        let about_item = MenuItem::with_label("About");
        about_item.connect_activate(|_| {
            show_about_dialog();
        });

        help_menu.append(&about_item);
        menu_bar.append(&help_menu_item);

        // Pack menu bar at the top of the main vbox
        main_vbox.pack_start(&menu_bar, false, false, 0);

        // ===== SCROLLABLE CONTENT AREA =====
        let scrolled_window = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_propagate_natural_width(true);

        let content_box = GtkBox::new(Orientation::Vertical, 12);
        content_box.set_margin_top(18);
        content_box.set_margin_bottom(18);
        content_box.set_margin_start(18);
        content_box.set_margin_end(18);

        // ===== CREATE NOTEBOOK (TABS) =====
        let notebook = Notebook::new();

        let output_tab = AudioTab::new(TabType::Output);
        let output_label = Label::new(Some("Output"));
        notebook.append_page(&output_tab.container, Some(&output_label));

        let input_tab = AudioTab::new(TabType::Input);
        let input_label = Label::new(Some("Input"));
        notebook.append_page(&input_tab.container, Some(&input_label));

        let advanced_tab = AdvancedTab::new();
        let advanced_label = Label::new(Some("Advanced"));
        notebook.append_page(&advanced_tab.container, Some(&advanced_label));

        let monitoring_tab = MonitoringTab::new();
        let monitoring_label = Label::new(Some("Monitor"));
        notebook.append_page(&monitoring_tab.container, Some(&monitoring_label));

        let config_inspector_tab = ConfigInspectorTab::new();
        let config_inspector_label = Label::new(Some("Config Inspector"));
        notebook.append_page(
            &config_inspector_tab.container,
            Some(&config_inspector_label),
        );

        content_box.pack_start(&notebook, true, true, 0);
        scrolled_window.add(&content_box);

        // Pack scrollable content below the menu bar
        main_vbox.pack_start(&scrolled_window, true, true, 0);

        // Add the main vbox to the window
        window.add(&main_vbox);

        let app_state = Self {
            window,
            notebook,
            output_tab,
            input_tab,
            advanced_tab,
            monitoring_tab,
            config_inspector_tab,
        };

        app_state.setup_signals();
        app_state.setup_advanced_signals();
        app_state.initialize_tabs();

        app_state
    }

    fn set_window_icon(window: &ApplicationWindow) {
        let icon_paths = [
            "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
            "icons/icon.png",
            "icon.png",
        ];

        for path in &icon_paths {
            if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(path) {
                window.set_icon(Some(&pixbuf));
                break;
            }
        }
    }

    fn initialize_tabs(&self) {
        self.output_tab.detect_all_devices();
        self.input_tab.detect_all_devices();
        self.output_tab.detect_current_settings();
        self.input_tab.detect_current_settings();
        self.output_tab.detect_current_device();
        self.input_tab.detect_current_device();
        self.advanced_tab.detect_advanced_devices();
    }

    fn setup_signals(&self) {
        self.output_tab.setup_signals(self.clone());
        self.input_tab.setup_signals(self.clone());

        let config_inspector_tab = self.config_inspector_tab.clone();
        self.config_inspector_tab
            .refresh_button
            .connect_clicked(move |_| {
                config_inspector_tab.scan_configs();
            });
    }

    fn setup_advanced_signals(&self) {
        self.advanced_tab.setup_signals(self.clone());
    }
}
