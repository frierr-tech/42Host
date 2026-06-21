//! Вкладка «Плагины» (§6.2.2, §10): сетка карточек, вкл/выкл/удаление, drag-and-drop.

use crate::ui;
use crate::window::AppWindow;
use adw::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct PluginsWidgets {
    pub root: gtk::Box,
    pub stack: gtk::Stack,
    pub grid: gtk::FlowBox,
    pub status: adw::StatusPage,
}

pub fn build() -> PluginsWidgets {
    let status = adw::StatusPage::new();
    status.set_icon_name(Some("application-x-addon-symbolic"));
    status.set_title("Нет плагинов");
    status.set_description(Some("Перетащите .jar сюда"));

    // Сетка карточек плагинов.
    let grid = gtk::FlowBox::new();
    grid.set_selection_mode(gtk::SelectionMode::None);
    grid.set_homogeneous(true);
    grid.set_min_children_per_line(2);
    grid.set_max_children_per_line(5);
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);
    grid.set_valign(gtk::Align::Start);
    grid.set_margin_top(12);
    grid.set_margin_bottom(12);
    grid.set_margin_start(12);
    grid.set_margin_end(12);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&grid));

    let stack = gtk::Stack::new();
    stack.add_named(&status, Some("empty"));
    stack.add_named(&scrolled, Some("list"));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);
    root.append(&stack);

    PluginsWidgets { root, stack, grid, status }
}

fn display_name(file: &str) -> String {
    file.trim_end_matches(".disabled").to_string()
}

fn is_disabled(file: &str) -> bool {
    file.ends_with(".disabled")
}

/// Базовое имя плагина без расширения, в нижнем регистре —
/// так называется папка его конфига: `Nullsleep.jar` -> `nullsleep` (§6.2.2).
fn config_folder_name(file: &str) -> String {
    file.trim_end_matches(".disabled")
        .trim_end_matches(".jar")
        .to_lowercase()
}

pub fn scan(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".jar") || name.ends_with(".jar.disabled") {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

pub fn refresh(aw: &AppWindow) {
    let pw = aw.plugins_widgets();
    while let Some(child) = pw.grid.first_child() {
        pw.grid.remove(&child);
    }

    let Some(server) = aw.current_server() else {
        pw.stack.set_visible_child_name("empty");
        return;
    };
    let dir = server.plugins_dir();
    let _ = std::fs::create_dir_all(&dir);
    let plugins = scan(&dir);

    if plugins.is_empty() {
        pw.stack.set_visible_child_name("empty");
        return;
    }
    pw.stack.set_visible_child_name("list");

    for path in plugins {
        let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        pw.grid.append(&build_card(aw, &dir, &path, &file));
    }
}

/// Карточка плагина в сетке.
fn build_card(aw: &AppWindow, plugins_dir: &Path, path: &Path, file: &str) -> gtk::FlowBoxChild {
    let disabled = is_disabled(file);

    let icon = gtk::Image::from_icon_name("application-x-java-archive-symbolic");
    icon.set_pixel_size(48);
    icon.set_margin_top(8);

    let title = gtk::Label::new(Some(&display_name(file)));
    title.add_css_class("heading");
    title.set_wrap(true);
    title.set_justify(gtk::Justification::Center);
    title.set_max_width_chars(18);
    title.set_lines(2);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let state = gtk::Label::new(Some(if disabled { "Выключен" } else { "Включён" }));
    state.add_css_class("dim-label");
    state.add_css_class("caption");

    // Кнопки действий.
    let toggle = gtk::Switch::new();
    toggle.set_active(!disabled);
    toggle.set_valign(gtk::Align::Center);
    toggle.set_tooltip_text(Some("Включить/выключить"));
    {
        let aw = aw.clone();
        let path = path.to_path_buf();
        toggle.connect_state_set(move |_, active| {
            let new_path = if active {
                PathBuf::from(path.to_string_lossy().trim_end_matches(".disabled").to_string())
            } else {
                let mut s = path.to_string_lossy().to_string();
                if !s.ends_with(".disabled") {
                    s.push_str(".disabled");
                }
                PathBuf::from(s)
            };
            if path != new_path {
                if let Err(e) = std::fs::rename(&path, &new_path) {
                    ui::toast(&aw.toasts(), &format!("Ошибка: {e}"));
                } else {
                    let aw = aw.clone();
                    glib::idle_add_local_once(move || refresh(&aw));
                }
            }
            glib::Propagation::Stop
        });
    }

    let cfg_btn = gtk::Button::from_icon_name("utilities-terminal-symbolic");
    cfg_btn.set_tooltip_text(Some("Открыть папку конфига в терминале"));
    cfg_btn.set_valign(gtk::Align::Center);
    cfg_btn.add_css_class("flat");
    {
        let aw = aw.clone();
        let dir = plugins_dir.to_path_buf();
        let folder = config_folder_name(file);
        cfg_btn.connect_clicked(move |_| open_config(&aw, &dir, &folder));
    }

    let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
    del_btn.set_tooltip_text(Some("Удалить"));
    del_btn.set_valign(gtk::Align::Center);
    del_btn.add_css_class("flat");
    {
        let aw = aw.clone();
        let path = path.to_path_buf();
        let name = display_name(file);
        del_btn.connect_clicked(move |_| {
            let aw2 = aw.clone();
            let path = path.clone();
            ui::confirm(
                &aw.window(),
                "Удалить плагин?",
                &format!("«{name}» будет перемещён в корзину."),
                "Удалить",
                true,
                move || {
                    let file = gio::File::for_path(&path);
                    match file.trash(gio::Cancellable::NONE) {
                        Ok(_) => {
                            let aw2 = aw2.clone();
                            glib::idle_add_local_once(move || refresh(&aw2));
                        }
                        Err(e) => ui::toast(&aw2.toasts(), &format!("Ошибка: {e}")),
                    }
                },
            );
        });
    }

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_halign(gtk::Align::Center);
    actions.set_margin_bottom(8);
    actions.append(&toggle);
    actions.append(&cfg_btn);
    actions.append(&del_btn);

    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.add_css_class("card");
    card.set_margin_start(2);
    card.set_margin_end(2);
    card.append(&icon);
    card.append(&title);
    card.append(&state);
    card.append(&actions);

    let child = gtk::FlowBoxChild::new();
    child.set_child(Some(&card));
    child
}

/// Открыть папку конфига плагина в терминале (§6.2.2).
fn open_config(aw: &AppWindow, plugins_dir: &Path, folder_name: &str) {
    let dir = plugins_dir.join(folder_name);
    if dir.is_dir() {
        if let Err(e) = crate::external::open_terminal_at(&aw.config(), &dir) {
            ui::toast(&aw.toasts(), &format!("Не удалось открыть терминал: {e}"));
        }
    } else {
        ui::toast(
            &aw.toasts(),
            &format!("Папка конфига «{folder_name}» не найдена (запустите сервер хотя бы раз)"),
        );
    }
}

/// Скопировать перетащенные .jar в plugins/ с разрешением коллизий (§10).
pub fn import_jars(aw: &AppWindow, files: &[PathBuf]) {
    let Some(server) = aw.current_server() else { return };
    let dir = server.plugins_dir();
    let _ = std::fs::create_dir_all(&dir);
    let mut count = 0;
    for src in files {
        if src.extension().and_then(|e| e.to_str()) != Some("jar") {
            continue;
        }
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("plugin");
        let mut dest = dir.join(format!("{stem}.jar"));
        let mut n = 1;
        while dest.exists() {
            dest = dir.join(format!("{stem}_{n}.jar"));
            n += 1;
        }
        if std::fs::copy(src, &dest).is_ok() {
            count += 1;
        }
    }
    if count > 0 {
        ui::toast(&aw.toasts(), &format!("Добавлено плагинов: {count}"));
        refresh(aw);
    }
}
