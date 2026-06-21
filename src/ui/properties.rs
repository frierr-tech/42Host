//! Вкладка «Настройки сервера» (§6.2.4): сетка параметров server.properties
//! с разделением на применяемые сразу и требующие перезапуска + JVM args.

use crate::ui;
use crate::window::AppWindow;
use adw::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// Ключи, которые можно применить на лету командой консоли (без перезапуска).
const LIVE_KEYS: &[&str] = &["difficulty", "white-list"];

const DIFFICULTY: &[&str] = &["peaceful", "easy", "normal", "hard"];
const GAMEMODE: &[&str] = &["survival", "creative", "adventure", "spectator"];

#[derive(Clone)]
enum PropLine {
    Pair(String, String),
    Raw(String),
}

#[derive(Clone)]
enum ValueWidget {
    Switch(gtk::Switch),
    Entry(gtk::Entry),
    Choice(gtk::DropDown, Vec<String>),
}

impl ValueWidget {
    fn value(&self) -> String {
        match self {
            ValueWidget::Switch(s) => if s.is_active() { "true".into() } else { "false".into() },
            ValueWidget::Entry(e) => e.text().to_string(),
            ValueWidget::Choice(d, opts) => {
                opts.get(d.selected() as usize).cloned().unwrap_or_default()
            }
        }
    }

    fn widget(&self) -> gtk::Widget {
        match self {
            ValueWidget::Switch(w) => w.clone().upcast(),
            ValueWidget::Entry(w) => w.clone().upcast(),
            ValueWidget::Choice(w, _) => w.clone().upcast(),
        }
    }
}

#[derive(Clone)]
pub struct PropertiesWidgets {
    pub root: gtk::Box,
    container: gtk::Box,
    entries: Rc<RefCell<Vec<(String, ValueWidget)>>>,
    lines: Rc<RefCell<Vec<PropLine>>>,
}

pub fn build() -> PropertiesWidgets {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 18);
    container.set_margin_top(12);
    container.set_margin_bottom(12);
    container.set_margin_start(12);
    container.set_margin_end(12);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&container));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);
    root.append(&scrolled);

    PropertiesWidgets {
        root,
        container,
        entries: Rc::new(RefCell::new(Vec::new())),
        lines: Rc::new(RefCell::new(Vec::new())),
    }
}

fn parse(path: &Path) -> Vec<PropLine> {
    let mut out = Vec::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                out.push(PropLine::Raw(line.to_string()));
            } else if let Some((k, v)) = line.split_once('=') {
                out.push(PropLine::Pair(k.trim().to_string(), v.to_string()));
            } else {
                out.push(PropLine::Raw(line.to_string()));
            }
        }
    }
    out
}

fn make_grid() -> gtk::FlowBox {
    let grid = gtk::FlowBox::new();
    grid.set_selection_mode(gtk::SelectionMode::None);
    grid.set_homogeneous(true);
    grid.set_min_children_per_line(2);
    grid.set_max_children_per_line(4);
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);
    grid.set_valign(gtk::Align::Start);
    grid
}

fn section_header(text: &str, subtitle: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let title = gtk::Label::new(Some(text));
    title.add_css_class("title-4");
    title.set_halign(gtk::Align::Start);
    let sub = gtk::Label::new(Some(subtitle));
    sub.add_css_class("dim-label");
    sub.add_css_class("caption");
    sub.set_halign(gtk::Align::Start);
    b.append(&title);
    b.append(&sub);
    b
}

/// Нормализовать числовое значение enum в имя (старый формат server.properties).
fn normalize(value: &str, options: &[&str]) -> String {
    if let Ok(idx) = value.trim().parse::<usize>() {
        if let Some(name) = options.get(idx) {
            return name.to_string();
        }
    }
    value.trim().to_string()
}

fn make_control(key: &str, value: &str) -> ValueWidget {
    match key {
        "difficulty" => choice(value, DIFFICULTY),
        "gamemode" => choice(value, GAMEMODE),
        _ if value == "true" || value == "false" => {
            let sw = gtk::Switch::new();
            sw.set_active(value == "true");
            sw.set_halign(gtk::Align::Center);
            ValueWidget::Switch(sw)
        }
        _ => {
            let e = gtk::Entry::new();
            e.set_text(value);
            e.set_hexpand(true);
            ValueWidget::Entry(e)
        }
    }
}

fn choice(value: &str, options: &[&str]) -> ValueWidget {
    let dd = gtk::DropDown::from_strings(options);
    let current = normalize(value, options);
    if let Some(idx) = options.iter().position(|o| *o == current) {
        dd.set_selected(idx as u32);
    }
    dd.set_halign(gtk::Align::Center);
    ValueWidget::Choice(dd, options.iter().map(|s| s.to_string()).collect())
}

fn make_card(key: &str, widget: &gtk::Widget) -> gtk::FlowBoxChild {
    let title = gtk::Label::new(Some(key));
    title.add_css_class("heading");
    title.set_wrap(true);
    title.set_max_width_chars(20);
    title.set_justify(gtk::Justification::Center);
    title.set_halign(gtk::Align::Center);

    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("card");
    card.set_margin_start(2);
    card.set_margin_end(2);
    card.set_margin_top(10);
    card.set_margin_bottom(10);
    let inner = gtk::Box::new(gtk::Orientation::Vertical, 8);
    inner.set_margin_start(10);
    inner.set_margin_end(10);
    inner.set_valign(gtk::Align::Center);
    inner.append(&title);
    widget.set_halign(gtk::Align::Center);
    inner.append(widget);
    card.append(&inner);

    let child = gtk::FlowBoxChild::new();
    child.set_child(Some(&card));
    child
}

pub fn refresh(aw: &AppWindow) {
    let pw = aw.properties_widgets();
    while let Some(child) = pw.container.first_child() {
        pw.container.remove(&child);
    }
    pw.entries.borrow_mut().clear();
    pw.lines.borrow_mut().clear();

    let Some(server) = aw.current_server() else { return };
    let path = server.properties_file();
    let lines = parse(&path);
    *pw.lines.borrow_mut() = lines.clone();

    // Панель действий сверху.
    pw.container.append(&action_bar(aw, &server, &path));

    if !path.exists() {
        let note = gtk::Label::new(Some(
            "Файл server.properties ещё не создан — появится после первого запуска сервера.",
        ));
        note.add_css_class("dim-label");
        note.set_halign(gtk::Align::Start);
        pw.container.append(&note);
    }

    let live_grid = make_grid();
    let restart_grid = make_grid();
    let mut live_count = 0;
    let mut restart_count = 0;

    for line in &lines {
        if let PropLine::Pair(k, v) = line {
            let widget = make_control(k, v);
            let card = make_card(k, &widget.widget());
            if LIVE_KEYS.contains(&k.as_str()) {
                live_grid.append(&card);
                live_count += 1;
            } else {
                restart_grid.append(&card);
                restart_count += 1;
            }
            pw.entries.borrow_mut().push((k.clone(), widget));
        }
    }

    if live_count > 0 {
        pw.container
            .append(&section_header("Применяются сразу", "Без перезапуска сервера"));
        pw.container.append(&live_grid);
    }
    if restart_count > 0 {
        pw.container.append(&section_header(
            "Требуют перезапуска",
            "Вступят в силу после перезапуска сервера",
        ));
        pw.container.append(&restart_grid);
    }

    // Группа JVM-аргументов.
    pw.container.append(&jvm_section(aw, &server));
}

fn action_bar(aw: &AppWindow, server: &crate::server::Server, path: &Path) -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    bar.set_halign(gtk::Align::End);

    let save_btn = gtk::Button::with_label("Сохранить");
    save_btn.add_css_class("suggested-action");
    {
        let aw = aw.clone();
        save_btn.connect_clicked(move |_| save(&aw));
    }

    let text_btn = gtk::Button::from_icon_name("document-edit-symbolic");
    text_btn.set_tooltip_text(Some("Открыть как текст"));
    {
        let aw = aw.clone();
        let path = path.to_path_buf();
        text_btn.connect_clicked(move |_| {
            if let Err(e) = crate::external::open_in_editor(&aw.config(), &path) {
                ui::toast(&aw.toasts(), &format!("Ошибка: {e}"));
            }
        });
    }

    let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
    folder_btn.set_tooltip_text(Some("Открыть папку сервера"));
    {
        let aw = aw.clone();
        let dir = server.path.clone();
        folder_btn.connect_clicked(move |_| {
            let _ = crate::external::open_in_file_manager(&aw.config(), &dir);
        });
    }

    bar.append(&folder_btn);
    bar.append(&text_btn);
    bar.append(&save_btn);
    bar
}

fn jvm_section(aw: &AppWindow, server: &crate::server::Server) -> gtk::Widget {
    let group = adw::PreferencesGroup::new();
    group.set_title("JVM аргументы");

    let jvm_row = adw::EntryRow::new();
    jvm_row.set_title("Аргументы (через пробел)");
    jvm_row.set_text(&server.jvm_args.join(" "));
    let java_row = adw::EntryRow::new();
    java_row.set_title("Путь к java");
    java_row.set_text(&server.java_path.to_string_lossy());
    group.add(&jvm_row);
    group.add(&java_row);

    let save_row = adw::ActionRow::new();
    save_row.set_title("Сохранить параметры запуска");
    let btn = gtk::Button::with_label("Сохранить");
    btn.add_css_class("suggested-action");
    btn.set_valign(gtk::Align::Center);
    {
        let aw = aw.clone();
        let id = server.id.clone();
        let jvm_row = jvm_row.clone();
        let java_row = java_row.clone();
        btn.connect_clicked(move |_| {
            let args: Vec<String> =
                jvm_row.text().split_whitespace().map(|s| s.to_string()).collect();
            let java = java_row.text().to_string();
            aw.update_server(&id, |s| {
                s.jvm_args = args.clone();
                s.java_path = java.clone().into();
            });
            ui::toast(&aw.toasts(), "Параметры запуска сохранены");
        });
    }
    save_row.add_suffix(&btn);
    group.add(&save_row);
    group.upcast()
}

/// Команды для применения «живых» параметров без перезапуска.
fn apply_live(aw: &AppWindow, key: &str, value: &str) {
    if !aw.current_running() {
        return;
    }
    let cmd = match key {
        "difficulty" => Some(format!("difficulty {value}")),
        "white-list" => Some(format!("whitelist {}", if value == "true" { "on" } else { "off" })),
        _ => None,
    };
    if let Some(cmd) = cmd {
        let _ = aw.send_command(&cmd);
    }
}

fn save(aw: &AppWindow) {
    let Some(server) = aw.current_server() else { return };
    let pw = aw.properties_widgets();
    let entries = pw.entries.borrow();
    let lines = pw.lines.borrow();

    let mut out = String::new();
    for line in lines.iter() {
        match line {
            PropLine::Raw(s) => {
                out.push_str(s);
                out.push('\n');
            }
            PropLine::Pair(k, original) => {
                let value = entries
                    .iter()
                    .find(|(ek, _)| ek == k)
                    .map(|(_, w)| w.value())
                    .unwrap_or_else(|| original.clone());
                out.push_str(k);
                out.push('=');
                out.push_str(&value);
                out.push('\n');
            }
        }
    }

    match std::fs::write(server.properties_file(), out) {
        Ok(_) => {
            // Применить «живые» параметры сразу.
            for (k, w) in entries.iter() {
                if LIVE_KEYS.contains(&k.as_str()) {
                    apply_live(aw, k, &w.value());
                }
            }
            let live = if aw.current_running() {
                " · сложность/whitelist применены на лету"
            } else {
                ""
            };
            ui::toast(&aw.toasts(), &format!("server.properties сохранён{live}"));
        }
        Err(e) => ui::toast(&aw.toasts(), &format!("Ошибка сохранения: {e}")),
    }
}
