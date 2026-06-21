//! Диалог «Настройки» (§7) — AdwPreferencesWindow.

use crate::config::Theme;
use crate::window::AppWindow;
use adw::prelude::*;

pub fn open(aw: &AppWindow) {
    let cfg = aw.config();
    let window = adw::PreferencesWindow::new();
    window.set_title(Some("Настройки"));
    window.set_transient_for(Some(&aw.window()));
    window.set_modal(true);
    window.set_search_enabled(false);

    // ---- 7.1 Внешний вид ----
    let appearance = adw::PreferencesPage::new();
    appearance.set_title("Внешний вид");
    appearance.set_icon_name(Some("applications-graphics-symbolic"));

    let look = adw::PreferencesGroup::new();
    look.set_title("Внешний вид");

    let theme_row = adw::ComboRow::new();
    theme_row.set_title("Тема");
    let themes = gtk::StringList::new(&["Системная", "Светлая", "Тёмная"]);
    theme_row.set_model(Some(&themes));
    theme_row.set_selected(match cfg.theme {
        Theme::System => 0,
        Theme::Light => 1,
        Theme::Dark => 2,
    });
    {
        let aw = aw.clone();
        theme_row.connect_selected_notify(move |row| {
            let theme = match row.selected() {
                1 => Theme::Light,
                2 => Theme::Dark,
                _ => Theme::System,
            };
            aw.update_config(|c| c.theme = theme);
            crate::application::apply_theme(theme);
        });
    }
    look.add(&theme_row);

    // Шрифт приложения (§4.2).
    let font_row = adw::ActionRow::new();
    font_row.set_title("Шрифт приложения");
    font_row.set_subtitle(cfg.font_family.as_deref().unwrap_or("Системный"));
    let font_btn = gtk::Button::with_label("Выбрать…");
    font_btn.set_valign(gtk::Align::Center);
    {
        let aw = aw.clone();
        let font_row = font_row.clone();
        font_btn.connect_clicked(move |_| {
            let dialog = gtk::FontDialog::new();
            let aw = aw.clone();
            let font_row = font_row.clone();
            dialog.choose_family(
                Some(&aw.window()),
                gtk::pango::FontFamily::NONE,
                gio::Cancellable::NONE,
                move |res| {
                    if let Ok(family) = res {
                        let name = family.name().to_string();
                        font_row.set_subtitle(&name);
                        aw.update_config(|c| c.font_family = Some(name.clone()));
                        aw.reapply_font();
                    }
                },
            );
        });
    }
    font_row.add_suffix(&font_btn);
    look.add(&font_row);

    // Размер шрифта консоли.
    let console_row = adw::SpinRow::with_range(10.0, 20.0, 1.0);
    console_row.set_title("Размер шрифта консоли");
    console_row.set_value(cfg.console_font_size as f64);
    {
        let aw = aw.clone();
        console_row.connect_value_notify(move |row| {
            let v = row.value() as u32;
            aw.update_config(|c| c.console_font_size = v);
        });
    }
    look.add(&console_row);
    appearance.add(&look);
    window.add(&appearance);

    // ---- 7.2 Внешние приложения ----
    let apps = adw::PreferencesPage::new();
    apps.set_title("Приложения");
    apps.set_icon_name(Some("applications-utilities-symbolic"));
    let ext = adw::PreferencesGroup::new();
    ext.set_title("Внешние приложения");
    ext.set_description(Some("Например: kitty, alacritty, foot · nvim, nano, code"));

    let term_row = adw::EntryRow::new();
    term_row.set_title("Терминал");
    term_row.set_text(&cfg.terminal);
    let editor_row = adw::EntryRow::new();
    editor_row.set_title("Редактор");
    editor_row.set_text(&cfg.editor);
    let fm_row = adw::EntryRow::new();
    fm_row.set_title("Файловый менеджер");
    fm_row.set_text(&cfg.file_manager);

    let preview = adw::ActionRow::new();
    preview.set_title("Команда запуска");
    preview.add_css_class("property");
    let update_preview = {
        let term_row = term_row.clone();
        let editor_row = editor_row.clone();
        let preview = preview.clone();
        move || {
            let t = term_row.text();
            let e = editor_row.text();
            let sep = if t == "kitty" || t == "foot" { "--" } else { "-e" };
            preview.set_subtitle(&format!("{t} {sep} {e} <file>"));
        }
    };
    update_preview();

    {
        let aw = aw.clone();
        let up = update_preview.clone();
        term_row.connect_changed(move |r| {
            let v = r.text().to_string();
            aw.update_config(|c| c.terminal = v.clone());
            up();
        });
    }
    {
        let aw = aw.clone();
        let up = update_preview.clone();
        editor_row.connect_changed(move |r| {
            let v = r.text().to_string();
            aw.update_config(|c| c.editor = v.clone());
            up();
        });
    }
    {
        let aw = aw.clone();
        fm_row.connect_changed(move |r| {
            let v = r.text().to_string();
            aw.update_config(|c| c.file_manager = v.clone());
        });
    }
    ext.add(&term_row);
    ext.add(&editor_row);
    ext.add(&fm_row);
    ext.add(&preview);
    apps.add(&ext);
    window.add(&apps);

    // ---- 7.3 Сервер ----
    let server = adw::PreferencesPage::new();
    server.set_title("Сервер");
    server.set_icon_name(Some("network-server-symbolic"));
    let srv = adw::PreferencesGroup::new();
    srv.set_title("Сервер");

    let java_row = adw::EntryRow::new();
    java_row.set_title("Путь к Java");
    java_row.set_text(&cfg.java_path);
    {
        let aw = aw.clone();
        java_row.connect_changed(move |r| {
            let v = r.text().to_string();
            aw.update_config(|c| c.java_path = v.clone());
        });
    }
    srv.add(&java_row);

    let auto_row = adw::SwitchRow::new();
    auto_row.set_title("Автоперезапуск при краше");
    auto_row.set_active(cfg.auto_restart);
    {
        let aw = aw.clone();
        auto_row.connect_active_notify(move |r| {
            let v = r.is_active();
            aw.update_config(|c| c.auto_restart = v);
        });
    }
    srv.add(&auto_row);

    let delay_row = adw::SpinRow::with_range(0.0, 120.0, 1.0);
    delay_row.set_title("Задержка перезапуска, сек");
    delay_row.set_value(cfg.restart_delay_secs as f64);
    {
        let aw = aw.clone();
        delay_row.connect_value_notify(move |r| {
            let v = r.value() as u32;
            aw.update_config(|c| c.restart_delay_secs = v);
        });
    }
    srv.add(&delay_row);

    let buf_row = adw::SpinRow::with_range(1000.0, 50000.0, 1000.0);
    buf_row.set_title("Размер буфера консоли, строк");
    buf_row.set_value(cfg.console_buffer_max as f64);
    {
        let aw = aw.clone();
        buf_row.connect_value_notify(move |r| {
            let v = r.value() as usize;
            aw.update_config(|c| c.console_buffer_max = v);
        });
    }
    srv.add(&buf_row);
    server.add(&srv);

    // ---- 7.4 Пути ----
    let paths = adw::PreferencesGroup::new();
    paths.set_title("Пути");
    let conf_row = adw::ActionRow::new();
    conf_row.set_title("Папка конфигурации 42Host");
    conf_row.set_subtitle(&crate::config::config_dir().to_string_lossy());
    let conf_btn = gtk::Button::from_icon_name("folder-open-symbolic");
    conf_btn.set_valign(gtk::Align::Center);
    {
        let aw = aw.clone();
        conf_btn.connect_clicked(move |_| {
            let _ = crate::external::open_in_file_manager(&aw.config(), &crate::config::config_dir());
        });
    }
    conf_row.add_suffix(&conf_btn);
    paths.add(&conf_row);
    server.add(&paths);
    window.add(&server);

    window.present();
}
