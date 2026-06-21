#![cfg_attr(windows, windows_subsystem = "windows")]

//! 42Host — кроссплатформенная точка входа.

mod application;
mod config;
mod external;
mod manager;
mod monitor;
mod server;
mod sidebar_row;
mod state;
mod ui;
mod window;

use gtk::prelude::*;

const APP_ID: &str = "io.github.whyoolw.Host42";

fn main() -> glib::ExitCode {
    env_logger::init();

    // Регистрируем встроенные ресурсы (style.css).
    gio::resources_register_include!("host42.gresource")
        .expect("не удалось зарегистрировать ресурсы");

    let app = application::build_application();
    app.run()
}
