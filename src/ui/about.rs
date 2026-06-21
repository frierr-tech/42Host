//! Диалог «О приложении» (§7.5).

use crate::APP_ID;
use adw::prelude::*;

pub fn open(parent: &impl IsA<gtk::Window>) {
    let about = adw::AboutWindow::new();
    about.set_transient_for(Some(parent));
    about.set_modal(true);
    about.set_application_name("42Host");
    about.set_application_icon(APP_ID);
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_comments("Приложение для запуска и управления серверами Minecraft");
    about.set_developers(&["claude & whyoolw"]);
    about.set_license_type(gtk::License::MitX11);
    about.set_website("https://github.com/whyoolw/42host");
    about.present();
}
