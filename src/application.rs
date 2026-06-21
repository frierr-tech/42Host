//! ADW::Application, глобальные действия и инициализация стилей/темы (§2.3, §4, §7).

use crate::config::{self, AppConfig, Theme};
use crate::window::AppWindow;
use crate::APP_ID;
use adw::prelude::*;
use gtk::gdk;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build_application() -> adw::Application {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        load_css();
    });

    let win_cell: Rc<RefCell<Option<AppWindow>>> = Rc::new(RefCell::new(None));

    app.connect_activate(move |app| {
        if let Some(w) = win_cell.borrow().as_ref() {
            w.present();
            return;
        }

        let config = config::load_config();
        apply_theme(config.theme);
        apply_font(&config);

        let app_window = AppWindow::new(app, config);
        setup_actions(app, &app_window);
        app_window.present();
        *win_cell.borrow_mut() = Some(app_window);
    });

    app
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/io/github/whyoolw/Host42/style.css");
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Применить тему через AdwStyleManager (§4.1, §7.1).
pub fn apply_theme(theme: Theme) {
    let manager = adw::StyleManager::default();
    let scheme = match theme {
        Theme::System => adw::ColorScheme::Default,
        Theme::Light => adw::ColorScheme::ForceLight,
        Theme::Dark => adw::ColorScheme::ForceDark,
    };
    manager.set_color_scheme(scheme);
}

/// Применить пользовательский шрифт через отдельный CSS-провайдер (§4.2).
pub fn apply_font(config: &AppConfig) {
    let Some(display) = gdk::Display::default() else { return };
    let provider = gtk::CssProvider::new();
    if let Some(font) = &config.font_family {
        if !font.trim().is_empty() {
            provider.load_from_string(&format!("* {{ font-family: \"{}\"; }}", font));
        }
    }
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

fn setup_actions(app: &adw::Application, win: &AppWindow) {
    // app.settings
    let settings = gio::SimpleAction::new("settings", None);
    {
        let win = win.clone();
        settings.connect_activate(move |_, _| win.open_settings());
    }
    app.add_action(&settings);

    // app.about
    let about = gio::SimpleAction::new("about", None);
    {
        let win = win.clone();
        about.connect_activate(move |_, _| win.open_about());
    }
    app.add_action(&about);

    // app.add-server
    let add = gio::SimpleAction::new("add-server", None);
    {
        let win = win.clone();
        add.connect_activate(move |_, _| win.start_add_server());
    }
    app.add_action(&add);

    // app.quit
    let quit = gio::SimpleAction::new("quit", None);
    {
        let app = app.clone();
        quit.connect_activate(move |_, _| app.quit());
    }
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<primary>q"]);
}
