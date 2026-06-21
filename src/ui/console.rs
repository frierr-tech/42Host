//! Вкладка «Консоль» (§6.2.1).

use crate::server::{LogKind, LogLine};
use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub struct ConsoleWidgets {
    pub root: gtk::Box,
    pub scroll: gtk::ScrolledWindow,
    pub view: gtk::TextView,
    pub buffer: gtk::TextBuffer,
    pub entry: gtk::Entry,
    pub send_btn: gtk::Button,
    pub clear_btn: gtk::Button,
    pub copy_btn: gtk::Button,
    pub scroll_btn: gtk::Button,
    /// Прилипание к низу для автопрокрутки (§6.2.1).
    pub stick: Rc<Cell<bool>>,
    /// История команд (последние 50) и текущая позиция.
    pub history: Rc<RefCell<Vec<String>>>,
    pub hist_pos: Rc<Cell<usize>>,
    end_mark: gtk::TextMark,
    tag_error: gtk::TextTag,
    tag_warn: gtk::TextTag,
    tag_chat: gtk::TextTag,
}

pub fn build(console_font_size: u32) -> ConsoleWidgets {
    let buffer = gtk::TextBuffer::new(None);

    // Теги для цветовой разметки. Цвета берём из темы через lookup_color,
    // чтобы подчиняться пользовательской теме GTK (§6.2.1).
    let tag_error = buffer.create_tag(Some("error"), &[]).unwrap();
    let tag_warn = buffer.create_tag(Some("warn"), &[]).unwrap();
    let tag_chat = buffer.create_tag(Some("chat"), &[]).unwrap();

    // Метка в конце буфера — для надёжной прокрутки вниз.
    let end_mark = buffer.create_mark(Some("scroll_end"), &buffer.end_iter(), false);

    let view = gtk::TextView::with_buffer(&buffer);
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.add_css_class("console-view");
    apply_font_size(&view, console_font_size);

    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_child(Some(&view));

    // Плавающая кнопка «прокрутить вниз» (§6.2.1).
    let scroll_btn = gtk::Button::from_icon_name("go-bottom-symbolic");
    scroll_btn.set_tooltip_text(Some("К последней строке"));
    scroll_btn.add_css_class("osd");
    scroll_btn.add_css_class("circular");
    scroll_btn.set_halign(gtk::Align::End);
    scroll_btn.set_valign(gtk::Align::End);
    scroll_btn.set_margin_end(16);
    scroll_btn.set_margin_bottom(16);
    scroll_btn.set_visible(false);

    let overlay = gtk::Overlay::new();
    overlay.set_vexpand(true);
    overlay.set_child(Some(&scroll));
    overlay.add_overlay(&scroll_btn);

    let stick = Rc::new(Cell::new(true));
    // Слежение за позицией прокрутки: прилипание/отклеивание + видимость кнопки.
    {
        let stick = stick.clone();
        let scroll_btn = scroll_btn.clone();
        let adj = scroll.vadjustment();
        adj.connect_value_changed(move |adj| {
            let at_bottom = adj.value() + adj.page_size() >= adj.upper() - 8.0;
            stick.set(at_bottom);
            scroll_btn.set_visible(!at_bottom);
        });
    }

    let entry = gtk::Entry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Команда серверу…"));

    let send_btn = gtk::Button::from_icon_name("media-playback-start-symbolic");
    send_btn.set_tooltip_text(Some("Отправить"));

    let input = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    input.set_margin_top(6);
    input.append(&entry);
    input.append(&send_btn);

    let clear_btn = gtk::Button::from_icon_name("edit-clear-all-symbolic");
    clear_btn.set_tooltip_text(Some("Очистить консоль"));
    let copy_btn = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy_btn.set_tooltip_text(Some("Скопировать выделение"));
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    toolbar.set_halign(gtk::Align::End);
    toolbar.append(&copy_btn);
    toolbar.append(&clear_btn);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
    root.set_margin_top(12);
    root.set_margin_bottom(12);
    root.set_margin_start(12);
    root.set_margin_end(12);
    root.append(&toolbar);
    root.append(&overlay);
    root.append(&input);

    let widgets = ConsoleWidgets {
        root,
        scroll,
        view,
        buffer,
        entry,
        send_btn,
        clear_btn,
        copy_btn,
        scroll_btn: scroll_btn.clone(),
        stick: stick.clone(),
        history: Rc::new(RefCell::new(Vec::new())),
        hist_pos: Rc::new(Cell::new(0)),
        end_mark,
        tag_error,
        tag_warn,
        tag_chat,
    };
    widgets.resolve_colors();

    // Клик по кнопке — прилипнуть и прокрутить вниз.
    {
        let w = widgets.clone();
        scroll_btn.connect_clicked(move |btn| {
            w.stick.set(true);
            w.scroll_to_bottom();
            btn.set_visible(false);
        });
    }

    widgets
}

fn apply_font_size(view: &gtk::TextView, size: u32) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!("textview {{ font-size: {}pt; }}", size));
    view.style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

impl ConsoleWidgets {
    /// Подтянуть цвета тегов из темы (вызывается при построении и смене темы).
    pub fn resolve_colors(&self) {
        let ctx = self.view.style_context();
        #[allow(deprecated)]
        let lookup = |name: &str| ctx.lookup_color(name);
        if let Some(c) = lookup("error_color").or_else(|| lookup("error_bg_color")) {
            self.tag_error.set_foreground_rgba(Some(&c));
        }
        if let Some(c) = lookup("warning_color").or_else(|| lookup("warning_bg_color")) {
            self.tag_warn.set_foreground_rgba(Some(&c));
        }
        if let Some(c) = lookup("dim_label_color").or_else(|| lookup("insensitive_fg_color")) {
            self.tag_chat.set_foreground_rgba(Some(&c));
        }
    }

    fn tag_for(&self, kind: LogKind) -> Option<&gtk::TextTag> {
        match kind {
            LogKind::Error => Some(&self.tag_error),
            LogKind::Warn => Some(&self.tag_warn),
            LogKind::Chat => Some(&self.tag_chat),
            LogKind::Info => None,
        }
    }

    pub fn append(&self, line: &LogLine) {
        let mut end = self.buffer.end_iter();
        let start_offset = end.offset();
        self.buffer.insert(&mut end, &line.text);
        self.buffer.insert(&mut self.buffer.end_iter(), "\n");
        if let Some(tag) = self.tag_for(line.kind) {
            let start = self.buffer.iter_at_offset(start_offset);
            let mut line_end = self.buffer.iter_at_offset(start_offset);
            line_end.forward_to_line_end();
            self.buffer.apply_tag(tag, &start, &line_end);
        }
        if self.stick.get() {
            self.scroll_to_bottom();
        }
    }

    pub fn set_lines<'a>(&self, lines: impl Iterator<Item = &'a LogLine>) {
        self.buffer.set_text("");
        for l in lines {
            self.append(l);
        }
        self.stick.set(true);
        self.scroll_btn.set_visible(false);
        self.scroll_to_bottom();
    }

    pub fn clear(&self) {
        self.buffer.set_text("");
    }

    /// Надёжная прокрутка к концу через метку (учитывает ещё не сделанный layout).
    pub fn scroll_to_bottom(&self) {
        let end = self.buffer.end_iter();
        self.buffer.move_mark(&self.end_mark, &end);
        self.view.scroll_to_mark(&self.end_mark, 0.0, true, 0.0, 1.0);
        // Подстраховка после обновления layout.
        let view = self.view.clone();
        let mark = self.end_mark.clone();
        glib::idle_add_local_once(move || {
            view.scroll_to_mark(&mark, 0.0, true, 0.0, 1.0);
        });
    }
}
