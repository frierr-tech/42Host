//! Основная панель сервера (§6): шапка + AdwViewStack с четырьмя вкладками.

use crate::ui::{console, players, plugins, properties};
use adw::prelude::*;

#[derive(Clone)]
pub struct ServerViewWidgets {
    pub root: gtk::Widget,
    pub avatar: adw::Avatar,
    pub avatar_button: gtk::Button,
    pub name_label: gtk::Label,
    pub path_label: gtk::Label,
    pub start_btn: gtk::Button,
    pub restart_btn: gtk::Button,
    pub stack: adw::ViewStack,
    pub switcher_bar: adw::ViewSwitcherBar,
    pub console: console::ConsoleWidgets,
    pub plugins: plugins::PluginsWidgets,
    pub players: players::PlayersWidgets,
    pub properties: properties::PropertiesWidgets,
}

pub fn build(console_font_size: u32) -> ServerViewWidgets {
    // --- Шапка (§6.1) ---
    let avatar = adw::Avatar::new(64, Some("S"), true);
    let avatar_button = gtk::Button::new();
    avatar_button.set_child(Some(&avatar));
    avatar_button.add_css_class("flat");
    avatar_button.add_css_class("circular");
    avatar_button.set_tooltip_text(Some("Сменить аватарку"));
    avatar_button.set_valign(gtk::Align::Center);

    let name_label = gtk::Label::new(Some("Сервер"));
    name_label.add_css_class("title-2");
    name_label.set_halign(gtk::Align::Start);

    let path_label = gtk::Label::new(Some(""));
    path_label.add_css_class("dim-label");
    path_label.set_halign(gtk::Align::Start);
    path_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);

    let title_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    title_box.set_valign(gtk::Align::Center);
    title_box.set_hexpand(true);
    title_box.append(&name_label);
    title_box.append(&path_label);

    let restart_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    restart_btn.set_tooltip_text(Some("Перезапуск"));
    restart_btn.set_valign(gtk::Align::Center);
    restart_btn.add_css_class("flat");

    let start_btn = gtk::Button::from_icon_name("media-playback-start-symbolic");
    start_btn.set_tooltip_text(Some("Запустить"));
    start_btn.set_valign(gtk::Align::Center);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_margin_top(16);
    header.set_margin_bottom(8);
    header.set_margin_start(16);
    header.set_margin_end(16);
    header.append(&avatar_button);
    header.append(&title_box);
    header.append(&restart_btn);
    header.append(&start_btn);

    // --- Вкладки (§6.2) ---
    let console = console::build(console_font_size);
    let plugins = plugins::build();
    let players = players::build();
    let properties = properties::build();

    let stack = adw::ViewStack::new();
    let p1 = stack.add_titled(&console.root, Some("console"), "Консоль");
    p1.set_icon_name(Some("utilities-terminal-symbolic"));
    let p2 = stack.add_titled(&plugins.root, Some("plugins"), "Плагины");
    p2.set_icon_name(Some("application-x-addon-symbolic"));
    let p3 = stack.add_titled(&players.root, Some("players"), "Игроки");
    p3.set_icon_name(Some("system-users-symbolic"));
    let p4 = stack.add_titled(&properties.root, Some("properties"), "Настройки");
    p4.set_icon_name(Some("emblem-system-symbolic"));
    stack.set_vexpand(true);

    let switcher_bar = adw::ViewSwitcherBar::new();
    switcher_bar.set_stack(Some(&stack));
    switcher_bar.set_reveal(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&header);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&stack);
    content.append(&switcher_bar);

    ServerViewWidgets {
        root: content.upcast(),
        avatar,
        avatar_button,
        name_label,
        path_label,
        start_btn,
        restart_btn,
        stack,
        switcher_bar,
        console,
        plugins,
        players,
        properties,
    }
}
