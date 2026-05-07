/*
 * Pro Audio Config - AudioApp Module
 * Version: 2.0
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

        // Set window icon from system or development location
        Self::set_window_icon(&window);

        // Create menu bar
        let menu_bar = Self::create_menu_bar();

        let scrolled_window = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_propagate_natural_width(true);

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_top(18);
        main_box.set_margin_bottom(18);
        main_box.set_margin_start(18);
        main_box.set_margin_end(18);

        // Add menu bar to main interface
        main_box.pack_start(&menu_bar, false, false, 0);

        // ===== CREATE NOTEBOOK (TABS) =====
        let notebook = Notebook::new();

        // Create output tab
        let output_tab = AudioTab::new(TabType::Output);
        let output_label = Label::new(Some("Output"));
        notebook.append_page(&output_tab.container, Some(&output_label));

        // Create input tab
        let input_tab = AudioTab::new(TabType::Input);
        let input_label = Label::new(Some("Input"));
        notebook.append_page(&input_tab.container, Some(&input_label));

        // Create advanced tab
        let advanced_tab = AdvancedTab::new();
        let advanced_label = Label::new(Some("Advanced"));
        notebook.append_page(&advanced_tab.container, Some(&advanced_label));

        // Create monitoring tab
        let monitoring_tab = MonitoringTab::new();
        let monitoring_label = Label::new(Some("Monitor"));
        notebook.append_page(&monitoring_tab.container, Some(&monitoring_label));

        // Create config-inspector tab
        let config_inspector_tab = ConfigInspectorTab::new();
        let config_inspector_label = Label::new(Some("Config Inspector"));
        notebook.append_page(
            &config_inspector_tab.container,
            Some(&config_inspector_label),
        );

        main_box.pack_start(&notebook, true, true, 0);
        scrolled_window.add(&main_box);
        window.add(&scrolled_window);

        let app_state = Self {
            window,
            notebook,
            output_tab,
            input_tab,
            advanced_tab,
            monitoring_tab,
            config_inspector_tab,
        };

        // ===== CONNECT SIGNALS =====
        app_state.setup_signals();

        // ===== CONNECT ADVANCED TAB SIGNALS =====
        app_state.setup_advanced_signals();

        // ===== DETECT ALL DEVICES AND CURRENT SETTINGS =====
        app_state.initialize_tabs();

        app_state
    }

    fn set_window_icon(window: &ApplicationWindow) {
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
            "/usr/share/icons/hicolor/48x48/apps/pro-audio-config.png",
            "/usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png",
        ];

        for path in &icon_paths {
            if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_file(path) {
                window.set_icon(Some(&pixbuf));
                break;
            }
        }
    }

    fn create_menu_bar() -> MenuBar {
        let menu_bar = MenuBar::new();

        // Create Help menu
        let help_menu = Menu::new();
        let help_menu_item = MenuItem::with_label("Help");
        help_menu_item.set_submenu(Some(&help_menu));

        // Create About item
        let about_item = MenuItem::with_label("About");
        about_item.connect_activate(|_| {
            show_about_dialog();
        });

        help_menu.append(&about_item);
        menu_bar.append(&help_menu_item);

        menu_bar
    }

    fn initialize_tabs(&self) {
        // Initialize both tabs with the same pattern
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
