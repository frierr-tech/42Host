//! Главное окно: боковое меню, панель сервера, управление процессами (§5, §6, §8).

use crate::config::{self, AppConfig};
use crate::manager::{self, ServerEvent};
use crate::server::{Server, ServerStatus};
use crate::sidebar_row::SidebarRow;
use crate::state::AppState;
use crate::ui::server_view::{self, ServerViewWidgets};
use crate::ui::{self, about, players, plugins, properties, settings};
use adw::prelude::*;
use gtk::gdk;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

pub struct Inner {
    app: adw::Application,
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    state: RefCell<AppState>,
    list: gtk::ListBox,
    split: adw::NavigationSplitView,
    content_stack: gtk::Stack,
    rows: RefCell<HashMap<String, SidebarRow>>,
    sv: ServerViewWidgets,
}

#[derive(Clone)]
pub struct AppWindow(Rc<Inner>);

impl AppWindow {
    pub fn new(app: &adw::Application, config: AppConfig) -> Self {
        let servers = config::load_servers();
        let state = AppState::new(config.clone(), servers);

        // --- Боковое меню (§5.1) ---
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        list.add_css_class("navigation-sidebar");

        let sidebar_scroll = gtk::ScrolledWindow::new();
        sidebar_scroll.set_vexpand(true);
        sidebar_scroll.set_child(Some(&list));

        let add_btn = gtk::Button::from_icon_name("list-add-symbolic");
        add_btn.set_tooltip_text(Some("Добавить сервер"));
        add_btn.set_action_name(Some("app.add-server"));

        let menu = gio::Menu::new();
        menu.append(Some("Настройки"), Some("app.settings"));
        menu.append(Some("О приложении"), Some("app.about"));
        let section = gio::Menu::new();
        section.append(Some("Добавить сервер"), Some("app.add-server"));
        menu.append_section(None, &section);

        let menu_btn = gtk::MenuButton::new();
        menu_btn.set_icon_name("view-more-symbolic");
        menu_btn.set_menu_model(Some(&menu));

        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.set_title_widget(Some(&adw::WindowTitle::new("42Host", "")));
        sidebar_header.pack_start(&add_btn);
        sidebar_header.pack_end(&menu_btn);

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.set_content(Some(&sidebar_scroll));

        let sidebar_page = adw::NavigationPage::new(&sidebar_toolbar, "Серверы");

        // --- Контент (§6) ---
        let sv = server_view::build(config.console_font_size);

        let empty = adw::StatusPage::new();
        empty.set_icon_name(Some("network-server-symbolic"));
        empty.set_title("Сервер не выбран");
        empty.set_description(Some("Добавьте сервер кнопкой + в боковом меню"));

        let content_stack = gtk::Stack::new();
        content_stack.add_named(&empty, Some("empty"));
        content_stack.add_named(&sv.root, Some("server"));
        content_stack.set_visible_child_name("empty");

        let content_header = adw::HeaderBar::new();
        content_header.set_title_widget(Some(&adw::WindowTitle::new("42Host", "")));

        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&content_header);
        content_toolbar.set_content(Some(&content_stack));

        let content_page = adw::NavigationPage::new(&content_toolbar, "42Host");

        let split = adw::NavigationSplitView::new();
        split.set_sidebar(Some(&sidebar_page));
        split.set_content(Some(&content_page));
        split.set_min_sidebar_width(260.0);

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&split));

        let window = adw::ApplicationWindow::new(app);
        window.set_title(Some("42Host"));
        window.set_default_size(1000, 680);
        window.set_content(Some(&toasts));

        // Адаптивность как у Nautilus: при сужении весь интерфейс сворачивается
        // в одноколоночный режим с выдвижным боковым меню (§4.1).
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            720.0,
            adw::LengthUnit::Px,
        ));
        {
            let split = split.clone();
            breakpoint.connect_apply(move |_| split.set_collapsed(true));
        }
        {
            let split = split.clone();
            breakpoint.connect_unapply(move |_| split.set_collapsed(false));
        }
        window.add_breakpoint(breakpoint);

        let inner = Rc::new(Inner {
            app: app.clone(),
            window,
            toasts,
            state: RefCell::new(state),
            list,
            split,
            content_stack,
            rows: RefCell::new(HashMap::new()),
            sv,
        });
        let aw = AppWindow(inner);
        aw.populate_sidebar();
        aw.wire_signals();
        aw.start_metrics_tick();
        aw
    }

    // ---- Доступ для UI-модулей ----
    pub fn present(&self) {
        self.0.window.present();
    }
    pub fn window(&self) -> adw::ApplicationWindow {
        self.0.window.clone()
    }
    pub fn toasts(&self) -> adw::ToastOverlay {
        self.0.toasts.clone()
    }
    pub fn config(&self) -> AppConfig {
        self.0.state.borrow().config.clone()
    }
    pub fn plugins_widgets(&self) -> plugins::PluginsWidgets {
        self.0.sv.plugins.clone()
    }
    pub fn players_widgets(&self) -> players::PlayersWidgets {
        self.0.sv.players.clone()
    }
    pub fn properties_widgets(&self) -> properties::PropertiesWidgets {
        self.0.sv.properties.clone()
    }
    pub fn current_server(&self) -> Option<Server> {
        let st = self.0.state.borrow();
        st.selected.as_ref().and_then(|id| st.server(id).cloned())
    }
    pub fn current_running(&self) -> bool {
        let st = self.0.state.borrow();
        st.selected
            .as_ref()
            .and_then(|id| st.runtime(id))
            .map(|r| r.status.is_active())
            .unwrap_or(false)
    }

    pub fn update_config(&self, f: impl FnOnce(&mut AppConfig)) {
        {
            let mut st = self.0.state.borrow_mut();
            f(&mut st.config);
            let _ = config::save_config(&st.config);
        }
    }
    pub fn reapply_font(&self) {
        crate::application::apply_font(&self.config());
    }

    pub fn update_server(&self, id: &str, f: impl FnOnce(&mut Server)) {
        let mut st = self.0.state.borrow_mut();
        if let Some(s) = st.server_mut(id) {
            f(s);
        }
        let _ = config::save_servers(&st.servers);
        let name = st.server(id).map(|s| s.name.clone());
        drop(st);
        if let (Some(row), Some(name)) = (self.0.rows.borrow().get(id), name) {
            row.set_name(&name);
        }
    }

    pub fn send_command(&self, cmd: &str) -> std::io::Result<()> {
        let st = self.0.state.borrow();
        let Some(id) = st.selected.clone() else {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "сервер не выбран"));
        };
        match st.runtime(&id).and_then(|r| r.handle.as_ref()) {
            Some(h) => h.send_command(cmd),
            None => Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "сервер не запущен")),
        }
    }

    pub fn open_settings(&self) {
        settings::open(self);
    }
    pub fn open_about(&self) {
        about::open(&self.0.window);
    }

    // ---- Боковое меню ----
    fn populate_sidebar(&self) {
        while let Some(child) = self.0.list.first_child() {
            self.0.list.remove(&child);
        }
        self.0.rows.borrow_mut().clear();

        let servers: Vec<Server> = self.0.state.borrow().servers.clone();
        for server in &servers {
            let row = SidebarRow::new(server);
            row.row.set_widget_name(&server.id);
            self.add_row_context_menu(&row, &server.id);
            self.0.list.append(&row.row);
            self.0.rows.borrow_mut().insert(server.id.clone(), row);
        }
    }

    fn add_row_context_menu(&self, row: &SidebarRow, id: &str) {
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gdk::BUTTON_SECONDARY);
        let aw = self.clone();
        let id = id.to_string();
        let row_widget = row.row.clone();
        gesture.connect_pressed(move |_, _, x, y| {
            aw.show_row_menu(&id, &row_widget, x, y);
        });
        row.row.add_controller(gesture);
    }

    fn show_row_menu(&self, id: &str, anchor: &gtk::ListBoxRow, x: f64, y: f64) {
        let popover = gtk::Popover::new();
        popover.set_parent(anchor);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.set_has_arrow(false);
        popover.set_halign(gtk::Align::Start);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let rename = menu_item("Переименовать");
        let remove = menu_item("Удалить из списка");
        let open = menu_item("Открыть папку");
        vbox.append(&rename);
        vbox.append(&open);
        vbox.append(&remove);
        popover.set_child(Some(&vbox));

        {
            let aw = self.clone();
            let id = id.to_string();
            let popover = popover.clone();
            rename.connect_clicked(move |_| {
                popover.popdown();
                aw.rename_server(&id);
            });
        }
        {
            let aw = self.clone();
            let id = id.to_string();
            let popover = popover.clone();
            open.connect_clicked(move |_| {
                popover.popdown();
                if let Some(s) = aw.0.state.borrow().server(&id) {
                    let _ = crate::external::open_in_file_manager(&aw.config(), &s.path);
                }
            });
        }
        {
            let aw = self.clone();
            let id = id.to_string();
            let popover = popover.clone();
            remove.connect_clicked(move |_| {
                popover.popdown();
                aw.remove_server(&id);
            });
        }
        popover.popup();
    }

    fn rename_server(&self, id: &str) {
        let current = self
            .0
            .state
            .borrow()
            .server(id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let aw = self.clone();
        let id = id.to_string();
        ui::prompt(&self.0.window, "Переименовать сервер", "Новое имя", &current, move |name| {
            if !name.trim().is_empty() {
                aw.update_server(&id, |s| s.name = name.trim().to_string());
                if aw.0.state.borrow().selected.as_deref() == Some(id.as_str()) {
                    aw.rebind_header();
                }
            }
        });
    }

    fn remove_server(&self, id: &str) {
        let name = self
            .0
            .state
            .borrow()
            .server(id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let aw = self.clone();
        let id = id.to_string();
        ui::confirm(
            &self.0.window,
            "Удалить из списка?",
            &format!("«{name}» будет удалён из 42Host. Файлы на диске останутся."),
            "Удалить",
            true,
            move || {
                {
                    let mut st = aw.0.state.borrow_mut();
                    if let Some(rt) = st.runtimes.get(&id) {
                        if let Some(h) = &rt.handle {
                            h.kill();
                        }
                    }
                    st.servers.retain(|s| s.id != id);
                    st.runtimes.remove(&id);
                    if st.selected.as_deref() == Some(id.as_str()) {
                        st.selected = None;
                    }
                    let _ = config::save_servers(&st.servers);
                }
                aw.populate_sidebar();
                aw.0.content_stack.set_visible_child_name("empty");
            },
        );
    }

    // ---- Сигналы ----
    fn wire_signals(&self) {
        // Выбор сервера в списке.
        {
            let aw = self.clone();
            self.0.list.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let id = row.widget_name().to_string();
                    if !id.is_empty() {
                        aw.select_server(&id);
                    }
                }
            });
        }

        // Кнопка запуска/остановки.
        {
            let aw = self.clone();
            self.0.sv.start_btn.connect_clicked(move |_| aw.toggle_running());
        }
        // Перезапуск.
        {
            let aw = self.clone();
            self.0.sv.restart_btn.connect_clicked(move |_| aw.restart_current());
        }

        // Консоль: отправка команды.
        {
            let aw = self.clone();
            let entry = self.0.sv.console.entry.clone();
            self.0.sv.console.send_btn.connect_clicked(move |_| aw.console_send(&entry));
        }
        {
            let aw = self.clone();
            self.0.sv.console.entry.connect_activate(move |entry| aw.console_send(entry));
        }
        // История команд по стрелкам.
        {
            let console = self.0.sv.console.clone();
            let key = gtk::EventControllerKey::new();
            key.connect_key_pressed(move |_, keyval, _, _| {
                let hist = console.history.borrow();
                if hist.is_empty() {
                    return glib::Propagation::Proceed;
                }
                match keyval {
                    gdk::Key::Up => {
                        let pos = console.hist_pos.get().saturating_sub(1);
                        console.hist_pos.set(pos);
                        console.entry.set_text(&hist[pos]);
                        glib::Propagation::Stop
                    }
                    gdk::Key::Down => {
                        let pos = console.hist_pos.get() + 1;
                        if pos >= hist.len() {
                            console.hist_pos.set(hist.len());
                            console.entry.set_text("");
                        } else {
                            console.hist_pos.set(pos);
                            console.entry.set_text(&hist[pos]);
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            });
            self.0.sv.console.entry.add_controller(key);
        }
        // Очистить / копировать.
        {
            let console = self.0.sv.console.clone();
            self.0.sv.console.clear_btn.connect_clicked(move |_| console.clear());
        }
        {
            let console = self.0.sv.console.clone();
            self.0.sv.console.copy_btn.connect_clicked(move |_| {
                if let Some((start, end)) = console.buffer.selection_bounds() {
                    let text = console.buffer.text(&start, &end, false);
                    console.view.clipboard().set_text(&text);
                }
            });
        }

        // Поиск игроков.
        {
            let aw = self.clone();
            self.0.sv.players.search.connect_search_changed(move |_| players::refresh(&aw));
        }
        // Кнопка обновления списка онлайн.
        {
            let aw = self.clone();
            self.0.sv.players.refresh_btn.connect_clicked(move |_| players::request_refresh(&aw));
        }

        // Смена аватарки по клику (§ правки).
        {
            let aw = self.clone();
            self.0.sv.avatar_button.connect_clicked(move |_| aw.choose_avatar());
        }

        // Обновление вкладок при переключении.
        {
            let aw = self.clone();
            self.0.sv.stack.connect_visible_child_name_notify(move |stack| {
                match stack.visible_child_name().as_deref() {
                    Some("plugins") => plugins::refresh(&aw),
                    Some("players") => players::request_refresh(&aw),
                    Some("properties") => properties::refresh(&aw),
                    _ => {}
                }
            });
        }

        // Drag-and-drop .jar в плагины (§10).
        {
            let aw = self.clone();
            let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
            drop.connect_drop(move |_, value, _, _| {
                if let Ok(list) = value.get::<gdk::FileList>() {
                    let files: Vec<PathBuf> = list.files().iter().filter_map(|f| f.path()).collect();
                    plugins::import_jars(&aw, &files);
                    true
                } else {
                    false
                }
            });
            self.0.sv.plugins.root.add_controller(drop);
        }
    }

    fn console_send(&self, entry: &gtk::Entry) {
        let text = entry.text().to_string();
        if text.trim().is_empty() {
            return;
        }
        match self.send_command(&text) {
            Ok(_) => {
                let console = &self.0.sv.console;
                let mut hist = console.history.borrow_mut();
                hist.push(text.clone());
                if hist.len() > 50 {
                    hist.remove(0);
                }
                console.hist_pos.set(hist.len());
                entry.set_text("");
            }
            Err(e) => ui::toast(&self.0.toasts, &format!("Не удалось: {e}")),
        }
    }

    // ---- Выбор и привязка ----
    fn select_server(&self, id: &str) {
        self.0.state.borrow_mut().selected = Some(id.to_string());
        self.0.content_stack.set_visible_child_name("server");
        self.rebind_header();

        // Перезалить консоль из буфера выбранного сервера.
        {
            let st = self.0.state.borrow();
            if let Some(rt) = st.runtime(id) {
                self.0.sv.console.set_lines(rt.console.iter());
            }
        }
        plugins::refresh(self);
        players::request_refresh(self);
        properties::refresh(self);
        self.refresh_controls();

        // На узком экране — показать контент.
        self.0.split.set_show_content(true);
    }

    fn rebind_header(&self) {
        let Some(server) = self.current_server() else { return };
        self.0.sv.name_label.set_text(&server.name);
        self.0.sv.path_label.set_text(&server.path.to_string_lossy());
        self.0.sv.avatar.set_text(Some(&server.name));
        if let Some(path) = &server.avatar_path {
            if let Ok(texture) = gdk::Texture::from_file(&gio::File::for_path(path)) {
                self.0.sv.avatar.set_custom_image(Some(&texture));
            }
        } else {
            self.0.sv.avatar.set_custom_image(gdk::Paintable::NONE);
        }
    }

    /// Выбор пользовательской аватарки сервера (≤ 10 МБ, любой размер картинки).
    /// AdwAvatar сам кадрирует изображение по центру в круг (cover), без растяжения.
    fn choose_avatar(&self) {
        let Some(server) = self.current_server() else { return };
        let id = server.id.clone();

        let dialog = gtk::FileDialog::new();
        dialog.set_title("Выберите аватарку (PNG/JPEG/…)");
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Изображения"));
        filter.add_pixbuf_formats();
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));

        let aw = self.clone();
        dialog.open(Some(&self.0.window), gio::Cancellable::NONE, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    aw.set_server_avatar(&id, path);
                }
            }
        });
    }

    fn set_server_avatar(&self, id: &str, path: PathBuf) {
        const MAX_BYTES: u64 = 10 * 1024 * 1024;
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_BYTES {
            ui::toast(&self.0.toasts, "Файл больше 10 МБ — выберите изображение поменьше");
            return;
        }
        // Проверяем, что файл — корректное изображение.
        if gdk::Texture::from_file(&gio::File::for_path(&path)).is_err() {
            ui::toast(&self.0.toasts, "Не удалось загрузить изображение");
            return;
        }

        self.update_server(id, |s| s.avatar_path = Some(path.clone()));
        if let Some(row) = self.0.rows.borrow().get(id) {
            row.set_avatar(Some(&path));
        }
        if self.0.state.borrow().selected.as_deref() == Some(id) {
            self.rebind_header();
        }
        ui::toast(&self.0.toasts, "Аватарка обновлена");
    }

    fn refresh_controls(&self) {
        let active = self.current_running();
        let start = &self.0.sv.start_btn;
        if active {
            start.set_icon_name("media-playback-stop-symbolic");
            start.set_tooltip_text(Some("Остановить"));
            start.remove_css_class("suggested-action");
            start.add_css_class("destructive-action");
        } else {
            start.set_icon_name("media-playback-start-symbolic");
            start.set_tooltip_text(Some("Запустить"));
            start.remove_css_class("destructive-action");
            start.add_css_class("suggested-action");
        }
        self.0.sv.restart_btn.set_sensitive(active);
    }

    // ---- Управление процессом (§8) ----
    fn toggle_running(&self) {
        if self.current_running() {
            let aw = self.clone();
            ui::confirm(
                &self.0.window,
                "Остановить сервер?",
                "Сервер будет корректно остановлен командой stop.",
                "Остановить",
                true,
                move || aw.stop_current(),
            );
        } else {
            self.start_current();
        }
    }

    fn start_current(&self) {
        let Some(server) = self.current_server() else { return };
        let id = server.id.clone();

        // Проверка EULA (§8.1).
        match manager::ensure_eula(&server, false) {
            Ok(true) => {}
            Ok(false) => {
                let aw = self.clone();
                let server2 = server.clone();
                ui::confirm(
                    &self.0.window,
                    "Принять EULA Minecraft?",
                    "Для запуска сервера нужно принять EULA (eula.txt → eula=true).",
                    "Принять и запустить",
                    false,
                    move || {
                        if manager::ensure_eula(&server2, true).is_ok() {
                            aw.do_spawn(&server2);
                        }
                    },
                );
                return;
            }
            Err(e) => {
                ui::toast(&self.0.toasts, &format!("Ошибка eula.txt: {e}"));
                return;
            }
        }
        let _ = id;
        self.do_spawn(&server);
    }

    fn do_spawn(&self, server: &Server) {
        let id = server.id.clone();
        let (handle, rx) = match manager::spawn_server(server) {
            Ok(v) => v,
            Err(e) => {
                ui::toast(&self.0.toasts, &format!("Не удалось запустить: {e}"));
                return;
            }
        };
        {
            let mut st = self.0.state.borrow_mut();
            let rt = st.runtime_mut(&id);
            rt.status = ServerStatus::Starting;
            rt.handle = Some(handle);
            rt.started_at = Some(Instant::now());
            rt.console.clear();
        }
        if let Some(row) = self.0.rows.borrow().get(&id) {
            row.set_status(ServerStatus::Starting);
        }
        self.refresh_controls();
        self.0.sv.console.clear();
        self.pump_events(id, rx);
    }

    fn pump_events(&self, id: String, rx: async_channel::Receiver<ServerEvent>) {
        let aw = self.clone();
        glib::spawn_future_local(async move {
            while let Ok(ev) = rx.recv().await {
                match ev {
                    ServerEvent::Log(line) => {
                        let max = aw.0.state.borrow().config.console_buffer_max;
                        let selected = aw.0.state.borrow().selected.as_deref() == Some(id.as_str());
                        // Переход Starting -> Running по строке "Done (".
                        if line.text.contains("Done (") {
                            aw.set_status(&id, ServerStatus::Running);
                        }
                        {
                            let mut st = aw.0.state.borrow_mut();
                            st.runtime_mut(&id).push_log(line.clone(), max);
                        }
                        if selected {
                            aw.0.sv.console.append(&line);
                        }
                    }
                    ServerEvent::Exited(code) => {
                        let status = if code.unwrap_or(0) == 0 {
                            ServerStatus::Stopped
                        } else {
                            ServerStatus::Crashed
                        };
                        {
                            let mut st = aw.0.state.borrow_mut();
                            let rt = st.runtime_mut(&id);
                            rt.status = status;
                            rt.handle = None;
                            rt.started_at = None;
                        }
                        aw.set_status(&id, status);
                        if aw.0.state.borrow().selected.as_deref() == Some(id.as_str()) {
                            aw.refresh_controls();
                        }
                        // Автоперезапуск при краше (§8.4).
                        let cfg = aw.0.state.borrow().config.clone();
                        if status == ServerStatus::Crashed && cfg.auto_restart {
                            aw.schedule_restart(id.clone(), cfg.restart_delay_secs);
                        }
                        break;
                    }
                }
            }
        });
    }

    fn schedule_restart(&self, id: String, delay: u32) {
        let aw = self.clone();
        glib::timeout_add_seconds_local_once(delay.max(1), move || {
            if let Some(server) = aw.0.state.borrow().server(&id).cloned() {
                aw.do_spawn(&server);
            }
        });
    }

    fn set_status(&self, id: &str, status: ServerStatus) {
        self.0.state.borrow_mut().runtime_mut(id).status = status;
        if let Some(row) = self.0.rows.borrow().get(id) {
            row.set_status(status);
        }
    }

    fn stop_current(&self) {
        let st = self.0.state.borrow();
        let Some(id) = st.selected.clone() else { return };
        if let Some(rt) = st.runtime(&id) {
            if let Some(h) = &rt.handle {
                let _ = h.stop_soft();
            }
        }
        drop(st);
        // Жёсткое завершение, если за 30 сек не остановился (§8.2).
        let aw = self.clone();
        glib::timeout_add_seconds_local_once(30, move || {
            let st = aw.0.state.borrow();
            if let Some(rt) = st.runtime(&id) {
                if rt.status.is_active() {
                    if let Some(h) = &rt.handle {
                        h.kill();
                    }
                }
            }
        });
    }

    fn restart_current(&self) {
        let st = self.0.state.borrow();
        let Some(id) = st.selected.clone() else { return };
        let server = st.server(&id).cloned();
        let was_running = st.runtime(&id).map(|r| r.status.is_active()).unwrap_or(false);
        if let Some(rt) = st.runtime(&id) {
            if let Some(h) = &rt.handle {
                let _ = h.stop_soft();
            }
        }
        drop(st);
        let Some(server) = server else { return };
        if !was_running {
            self.do_spawn(&server);
            return;
        }
        // Подождать выхода процесса, затем запустить снова.
        let aw = self.clone();
        glib::timeout_add_seconds_local_once(5, move || {
            let active = aw
                .0
                .state
                .borrow()
                .runtime(&server.id)
                .map(|r| r.status.is_active())
                .unwrap_or(false);
            if active {
                if let Some(rt) = aw.0.state.borrow().runtime(&server.id) {
                    if let Some(h) = &rt.handle {
                        h.kill();
                    }
                }
            }
            aw.do_spawn(&server);
        });
    }

    // ---- Метрики (§9) ----
    fn start_metrics_tick(&self) {
        let aw = self.clone();
        glib::timeout_add_seconds_local(1, move || {
            aw.tick_metrics();
            glib::ControlFlow::Continue
        });
    }

    fn tick_metrics(&self) {
        let ids: Vec<String> = self.0.state.borrow().servers.iter().map(|s| s.id.clone()).collect();
        for id in ids {
            let (status, ram, cpu, uptime) = {
                let mut st = self.0.state.borrow_mut();
                let pid = st.runtime(&id).and_then(|r| r.pid());
                if let Some(pid) = pid {
                    if let Some(sample) = st.monitor.sample(pid) {
                        st.runtime_mut(&id).last_sample = sample;
                    }
                }
                let rt = st.runtime_mut(&id);
                (rt.status, rt.last_sample.ram_mb, rt.last_sample.cpu_percent, rt.uptime_secs())
            };
            if let Some(row) = self.0.rows.borrow().get(&id) {
                row.set_status(status);
                row.set_metrics(status, ram, cpu, uptime);
            }
        }
    }

    // ---- Добавление сервера (§5.3) ----
    pub fn start_add_server(&self) {
        let dialog = gtk::FileDialog::new();
        dialog.set_title("Выберите папку сервера");
        if let Some(dir) = &self.config().default_server_dir {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let aw = self.clone();
        dialog.select_folder(Some(&self.0.window), gio::Cancellable::NONE, move |res| {
            if let Ok(folder) = res {
                if let Some(path) = folder.path() {
                    aw.handle_selected_folder(path);
                }
            }
        });
    }

    fn handle_selected_folder(&self, path: PathBuf) {
        let jars: Vec<PathBuf> = std::fs::read_dir(&path)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jar"))
                    .collect()
            })
            .unwrap_or_default();

        match jars.len() {
            0 => ui::toast(&self.0.toasts, "В папке не найден .jar файл сервера"),
            1 => self.finalize_add(path, jars[0].clone()),
            _ => self.choose_jar(path, jars),
        }
    }

    fn choose_jar(&self, path: PathBuf, jars: Vec<PathBuf>) {
        let names: Vec<String> = jars
            .iter()
            .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string())
            .collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        let combo = gtk::DropDown::from_strings(&name_refs);

        let dialog = adw::MessageDialog::new(
            Some(&self.0.window),
            Some("Выберите ядро сервера"),
            Some("В папке несколько .jar — выберите нужный."),
        );
        dialog.set_extra_child(Some(&combo));
        dialog.add_response("cancel", "Отмена");
        dialog.add_response("ok", "Добавить");
        dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        let aw = self.clone();
        dialog.connect_response(None, move |d, resp| {
            if resp == "ok" {
                let idx = combo.selected() as usize;
                if let Some(jar) = jars.get(idx) {
                    aw.finalize_add(path.clone(), jar.clone());
                }
            }
            d.close();
        });
        dialog.present();
    }

    fn finalize_add(&self, path: PathBuf, jar: PathBuf) {
        let default_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("server")
            .to_string();
        let mut server = Server::new(default_name, path, jar);
        server.java_path = self.config().java_path.into();

        let id = server.id.clone();
        {
            let mut st = self.0.state.borrow_mut();
            st.servers.push(server);
            st.runtimes.insert(id.clone(), Default::default());
            let _ = config::save_servers(&st.servers);
        }
        self.populate_sidebar();
        // Выбрать только что добавленный сервер.
        if let Some(row) = self.0.rows.borrow().get(&id) {
            self.0.list.select_row(Some(&row.row));
        }
        ui::toast(&self.0.toasts, "Сервер добавлен");
    }
}

fn menu_item(label: &str) -> gtk::Button {
    let btn = gtk::Button::with_label(label);
    btn.add_css_class("flat");
    btn.set_halign(gtk::Align::Fill);
    if let Some(child) = btn.child() {
        child.set_halign(gtk::Align::Start);
    }
    btn
}
