//! Вкладка «Игроки» (§6.2.3): онлайн-список + действия через консоль.

use crate::ui;
use crate::window::AppWindow;
use adw::prelude::*;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone)]
pub struct PlayersWidgets {
    pub root: gtk::Box,
    pub stack: gtk::Stack,
    pub list: gtk::ListBox,
    pub status: adw::StatusPage,
    pub search: gtk::SearchEntry,
    pub refresh_btn: gtk::Button,
}

pub fn build() -> PlayersWidgets {
    let status = adw::StatusPage::new();
    status.set_icon_name(Some("system-users-symbolic"));
    status.set_title("Сервер пуст");
    status.set_description(Some("Нет игроков онлайн или сервер не запущен"));

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.set_valign(gtk::Align::Start);

    let clamp = adw::Clamp::new();
    clamp.set_child(Some(&list));
    clamp.set_margin_top(12);
    clamp.set_margin_bottom(12);
    clamp.set_margin_start(12);
    clamp.set_margin_end(12);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&clamp));

    let stack = gtk::Stack::new();
    stack.add_named(&status, Some("empty"));
    stack.add_named(&scrolled, Some("list"));

    let search = gtk::SearchEntry::new();
    search.set_hexpand(true);
    search.set_placeholder_text(Some("Поиск по нику…"));

    let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.set_tooltip_text(Some("Обновить список онлайн"));

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    toolbar.set_margin_top(12);
    toolbar.set_margin_start(12);
    toolbar.set_margin_end(12);
    toolbar.append(&search);
    toolbar.append(&refresh_btn);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);
    root.append(&toolbar);
    root.append(&stack);

    PlayersWidgets { root, stack, list, status, search, refresh_btn }
}

/// Парсинг онлайн-игроков по latest.log: учитывает join/left и ответы `list`.
pub fn parse_online(log: &Path) -> Vec<String> {
    let mut online: BTreeSet<String> = BTreeSet::new();
    let Ok(content) = std::fs::read_to_string(log) else {
        return Vec::new();
    };
    for line in content.lines() {
        if let Some(idx) = line.find("players online:") {
            // Авторитетный ответ команды `list` — заменяет текущий набор.
            online.clear();
            let rest = &line[idx + "players online:".len()..];
            for name in rest.split(',') {
                let n = name.trim().trim_end_matches('.').trim();
                if !n.is_empty() {
                    online.insert(n.to_string());
                }
            }
        } else if let Some(name) = extract_name(line, " joined the game") {
            online.insert(name);
        } else if let Some(name) = extract_name(line, " left the game") {
            online.remove(&name);
        }
    }
    online.into_iter().collect()
}

fn extract_name(line: &str, marker: &str) -> Option<String> {
    let idx = line.find(marker)?;
    let before = &line[..idx];
    let after_meta = before.rsplit("]: ").next().unwrap_or(before);
    let name = after_meta.trim().rsplit(' ').next().unwrap_or("").trim();
    if name.is_empty() || name.contains('[') {
        None
    } else {
        Some(name.to_string())
    }
}

/// Запросить свежий список у сервера (`list`) и обновить через мгновение.
pub fn request_refresh(aw: &AppWindow) {
    if aw.current_running() {
        let _ = aw.send_command("list");
        let aw = aw.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            refresh(&aw);
        });
    }
    refresh(aw);
}

pub fn refresh(aw: &AppWindow) {
    let pw = aw.players_widgets();
    while let Some(child) = pw.list.first_child() {
        pw.list.remove(&child);
    }

    let Some(server) = aw.current_server() else {
        pw.status.set_description(Some("Сервер не выбран"));
        pw.stack.set_visible_child_name("empty");
        return;
    };

    if !aw.current_running() {
        pw.status.set_description(Some("Сервер не запущен"));
        pw.stack.set_visible_child_name("empty");
        return;
    }

    let players = parse_online(&server.latest_log());
    let filter = pw.search.text().to_lowercase();
    let players: Vec<String> = players
        .into_iter()
        .filter(|p| filter.is_empty() || p.to_lowercase().contains(&filter))
        .collect();

    if players.is_empty() {
        pw.status.set_description(Some("Нет игроков онлайн — нажмите «Обновить»"));
        pw.stack.set_visible_child_name("empty");
        return;
    }
    pw.stack.set_visible_child_name("list");

    for nick in players {
        let row = adw::ActionRow::new();
        row.set_title(&nick);

        let avatar = adw::Avatar::new(32, Some(&nick), true);
        row.add_prefix(&avatar);

        let kick = action_button(aw, &nick, "Кик", "go-jump-symbolic", "kick");
        let ban = action_button(aw, &nick, "Бан", "action-unavailable-symbolic", "ban");
        let op = action_button(aw, &nick, "OP", "starred-symbolic", "op");
        row.add_suffix(&op);
        row.add_suffix(&kick);
        row.add_suffix(&ban);

        pw.list.append(&row);
    }
}

fn action_button(aw: &AppWindow, nick: &str, tooltip: &str, icon: &str, command: &str) -> gtk::Button {
    let btn = gtk::Button::from_icon_name(icon);
    btn.set_tooltip_text(Some(tooltip));
    btn.set_valign(gtk::Align::Center);
    btn.add_css_class("flat");
    let aw = aw.clone();
    let nick = nick.to_string();
    let command = command.to_string();
    btn.connect_clicked(move |_| {
        let full = format!("{command} {nick}");
        if let Err(e) = aw.send_command(&full) {
            ui::toast(&aw.toasts(), &format!("Не удалось: {e}"));
        } else {
            ui::toast(&aw.toasts(), &format!("Отправлено: {full}"));
        }
    });
    btn
}
