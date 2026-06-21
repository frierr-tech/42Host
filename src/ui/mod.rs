//! UI-слой: каркас панели сервера, вкладки и диалоги.

pub mod about;
pub mod console;
pub mod players;
pub mod plugins;
pub mod properties;
pub mod server_view;
pub mod settings;

use adw::prelude::*;

/// Показать тост в overlay.
pub fn toast(overlay: &adw::ToastOverlay, text: &str) {
    overlay.add_toast(adw::Toast::new(text));
}

/// Диалог подтверждения с деструктивной кнопкой. Вызывает `on_confirm` при «Да».
pub fn confirm(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    body: &str,
    confirm_label: &str,
    destructive: bool,
    on_confirm: impl Fn() + 'static,
) {
    let dialog = adw::MessageDialog::new(Some(parent), Some(title), Some(body));
    dialog.add_response("cancel", "Отмена");
    dialog.add_response("ok", confirm_label);
    if destructive {
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Destructive);
    } else {
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    }
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.connect_response(None, move |d, resp| {
        if resp == "ok" {
            on_confirm();
        }
        d.close();
    });
    dialog.present();
}

/// Простой диалог ввода одной строки. Вызывает `on_ok(text)`.
pub fn prompt(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    placeholder: &str,
    initial: &str,
    on_ok: impl Fn(String) + 'static,
) {
    let dialog = adw::MessageDialog::new(Some(parent), Some(title), None);
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.set_text(initial);
    entry.set_margin_top(8);
    entry.set_margin_bottom(8);
    entry.set_margin_start(12);
    entry.set_margin_end(12);
    dialog.set_extra_child(Some(&entry));
    dialog.add_response("cancel", "Отмена");
    dialog.add_response("ok", "ОК");
    dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("ok"));
    dialog.set_close_response("cancel");
    dialog.connect_response(None, move |d, resp| {
        if resp == "ok" {
            on_ok(entry.text().to_string());
        }
        d.close();
    });
    dialog.present();
}
