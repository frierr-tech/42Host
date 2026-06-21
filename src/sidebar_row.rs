//! Строка сервера в боковом меню (§5.2). Без GObject-subclass — плотный
//! набор виджетов со ссылками для живого обновления метрик.

use crate::monitor;
use crate::server::{Server, ServerStatus};
use adw::prelude::*;
use std::path::Path;

#[derive(Clone)]
pub struct SidebarRow {
    pub row: gtk::ListBoxRow,
    pub dot: gtk::Box,
    pub avatar: adw::Avatar,
    pub name: gtk::Label,
    pub summary_ram: gtk::Label,
    pub summary_cpu: gtk::Label,
    pub server_id: String,
}

impl SidebarRow {
    pub fn new(server: &Server) -> Self {
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("status-dot");
        dot.add_css_class("stopped");
        dot.set_valign(gtk::Align::Center);

        let avatar = adw::Avatar::new(40, Some(&server.name), true);
        if let Some(path) = &server.avatar_path {
            if let Ok(texture) = gtk::gdk::Texture::from_file(&gio::File::for_path(path)) {
                avatar.set_custom_image(Some(&texture));
            }
        }

        let name = gtk::Label::new(Some(&server.name));
        name.add_css_class("title");
        name.set_halign(gtk::Align::Start);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);

        // Метрики в две строки, чтобы не выходить за рамки (§5.2).
        let summary_ram = gtk::Label::new(Some("RAM: --"));
        summary_ram.add_css_class("dim-label");
        summary_ram.add_css_class("server-summary");
        summary_ram.set_halign(gtk::Align::Start);
        summary_ram.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let summary_cpu = gtk::Label::new(Some("CPU: --  ·  --:--:--"));
        summary_cpu.add_css_class("dim-label");
        summary_cpu.add_css_class("server-summary");
        summary_cpu.set_halign(gtk::Align::Start);
        summary_cpu.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let text_box = gtk::Box::new(gtk::Orientation::Vertical, 1);
        text_box.set_hexpand(true);
        text_box.set_valign(gtk::Align::Center);
        text_box.append(&name);
        text_box.append(&summary_ram);
        text_box.append(&summary_cpu);

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.append(&dot);
        hbox.append(&avatar);
        hbox.append(&text_box);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&hbox));

        Self {
            row,
            dot,
            avatar,
            name,
            summary_ram,
            summary_cpu,
            server_id: server.id.clone(),
        }
    }

    pub fn set_status(&self, status: ServerStatus) {
        for cls in ["stopped", "running", "starting", "crashed"] {
            self.dot.remove_css_class(cls);
        }
        self.dot.add_css_class(status.css_class());
    }

    /// Обновить сводку метрик в две строки (§5.2).
    pub fn set_metrics(&self, status: ServerStatus, ram_mb: f64, cpu: f32, uptime_secs: u64) {
        if status.is_active() {
            self.summary_ram.set_text(&format!("RAM: {}", monitor::format_ram(ram_mb)));
            self.summary_cpu
                .set_text(&format!("CPU: {:.0}%  ·  {}", cpu, monitor::format_uptime(uptime_secs)));
        } else {
            self.summary_ram.set_text("RAM: --");
            self.summary_cpu.set_text("CPU: --  ·  --:--:--");
        }
    }

    pub fn set_name(&self, name: &str) {
        self.name.set_text(name);
        self.avatar.set_text(Some(name));
    }

    pub fn set_avatar(&self, path: Option<&Path>) {
        match path {
            Some(p) => {
                if let Ok(texture) = gtk::gdk::Texture::from_file(&gio::File::for_path(p)) {
                    self.avatar.set_custom_image(Some(&texture));
                }
            }
            None => self.avatar.set_custom_image(gtk::gdk::Paintable::NONE),
        }
    }
}
