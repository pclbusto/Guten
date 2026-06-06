use gtk::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::annotations::AnnotationStore;
use crate::document::DocumentModel;
use crate::settings::ReaderSettings;
use crate::view::BookView;

pub struct GutenReaderApp {
    pub window: libadwaita::ApplicationWindow,
    document: Rc<RefCell<Option<DocumentModel>>>,
    settings: Rc<RefCell<ReaderSettings>>,
    annotations: Rc<RefCell<Option<AnnotationStore>>>,
    book_view: Rc<BookView>,
    toc_entries: Rc<RefCell<Vec<String>>>, // hrefs del TOC
    title_label: gtk::Label,
    toc_list: gtk::ListBox,
    progress_label: gtk::Label,
    header_bar: libadwaita::HeaderBar,
    bottom_bar: gtk::Box,
    overlay: libadwaita::OverlaySplitView,
    toc_btn: gtk::ToggleButton,
    focus_btn: gtk::ToggleButton,
    two_page_btn: gtk::ToggleButton,
    voice_btn: gtk::MenuButton,
    annotate_btn: gtk::MenuButton,
    toast_overlay: libadwaita::ToastOverlay,
    self_weak: RefCell<std::rc::Weak<Self>>,
    tts_engine: Rc<RefCell<Option<crate::tts::TtsEngine>>>,
}

impl GutenReaderApp {
    pub fn new(app: &libadwaita::Application) -> Rc<Self> {
        let settings = Rc::new(RefCell::new(ReaderSettings::load()));
        let document: Rc<RefCell<Option<DocumentModel>>> = Rc::new(RefCell::new(None));
        let annotations: Rc<RefCell<Option<AnnotationStore>>> = Rc::new(RefCell::new(None));
        let toc_entries: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

        // ---- Widgets ----
        let book_view = Rc::new(BookView::new());

        let title_label = gtk::Label::builder()
            .label("GutenReader")
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(40)
            .build();

        let header_bar = libadwaita::HeaderBar::builder().build();
        header_bar.set_title_widget(Some(&title_label));

        let open_btn = gtk::Button::from_icon_name("document-open-symbolic");
        open_btn.set_tooltip_text(Some("Abrir EPUB (Ctrl+O)"));
        header_bar.pack_start(&open_btn);

        let search_btn = gtk::Button::from_icon_name("folder-saved-search-symbolic");
        search_btn.set_tooltip_text(Some("Buscar (Ctrl+F)"));
        header_bar.pack_start(&search_btn);

        let toc_btn = gtk::ToggleButton::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text("Tabla de contenidos (F9)")
            .build();
        header_bar.pack_start(&toc_btn);

        let focus_btn = gtk::ToggleButton::builder()
            .icon_name("view-fullscreen-symbolic")
            .tooltip_text("Modo foco")
            .build();
        header_bar.pack_end(&focus_btn);

        let two_page_btn = gtk::ToggleButton::builder()
            .icon_name("view-dual-symbolic")
            .tooltip_text("Modo dos páginas")
            .active(settings.borrow().two_page_mode)
            .build();
        header_bar.pack_end(&two_page_btn);

        let theme_btn = gtk::MenuButton::builder()
            .icon_name("weather-clear-night-symbolic")
            .tooltip_text("Cambiar tema")
            .build();
        let theme_menu = gtk::gio::Menu::new();
        theme_menu.append(Some("Día"), Some("app.theme_day"));
        theme_menu.append(Some("Noche"), Some("app.theme_night"));
        theme_menu.append(Some("Sepia"), Some("app.theme_sepia"));
        theme_btn.set_menu_model(Some(&theme_menu));
        header_bar.pack_end(&theme_btn);

        let voice_btn = gtk::MenuButton::builder()
            .icon_name("audio-headphones-symbolic")
            .tooltip_text("Seleccionar voz TTS")
            .build();
        let voice_menu = gtk::gio::Menu::new();
        // Las voces se agregarán dinámicamente
        voice_btn.set_menu_model(Some(&voice_menu));
        header_bar.pack_end(&voice_btn);

        let annotate_btn = gtk::MenuButton::builder()
            .icon_name("edit-select-all-symbolic")
            .tooltip_text("Anotar selección (Ctrl+Shift+A)")
            .build();
        header_bar.pack_end(&annotate_btn);

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menú")
            .build();
        let app_menu = gtk::gio::Menu::new();
        app_menu.append(Some("Atajos de teclado"), Some("win.show-shortcuts"));
        app_menu.append(Some("Salir"), Some("app.quit"));
        menu_btn.set_menu_model(Some(&app_menu));
        header_bar.pack_end(&menu_btn);

        let tts_btn = gtk::Button::from_icon_name("audio-speakers-symbolic");
        tts_btn.set_tooltip_text(Some("Leer en voz alta (F5)"));

        let progress_label = gtk::Label::new(Some("- / -"));

        let bottom_bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        let prev_chap_btn = gtk::Button::from_icon_name("go-previous-symbolic");
        prev_chap_btn.set_tooltip_text(Some("Capítulo anterior (Ctrl+←)"));
        let next_chap_btn = gtk::Button::from_icon_name("go-next-symbolic");
        next_chap_btn.set_tooltip_text(Some("Capítulo siguiente (Ctrl+→)"));
        let prev_page_btn = gtk::Button::from_icon_name("go-left-symbolic");
        prev_page_btn.set_tooltip_text(Some("Página anterior (←)"));
        let next_page_btn = gtk::Button::from_icon_name("go-right-symbolic");
        next_page_btn.set_tooltip_text(Some("Página siguiente (→)"));

        bottom_bar.append(&prev_chap_btn);
        bottom_bar.append(&prev_page_btn);
        bottom_bar.append(&gtk::Box::builder().hexpand(true).build());
        bottom_bar.append(&tts_btn);
        bottom_bar.append(&progress_label);
        bottom_bar.append(&gtk::Box::builder().hexpand(true).build());
        bottom_bar.append(&next_page_btn);
        bottom_bar.append(&next_chap_btn);

        // TOC panel
        let toc_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .css_classes(vec!["navigation-sidebar".to_string()])
            .build();
        let toc_scroll = gtk::ScrolledWindow::builder()
            .child(&toc_list)
            .hexpand(true)
            .vexpand(true)
            .build();
        let toc_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(260)
            .build();
        let toc_header = gtk::Label::builder()
            .label("Contenido")
            .margin_top(12)
            .margin_bottom(12)
            .css_classes(vec!["heading".to_string()])
            .build();
        toc_box.append(&toc_header);
        toc_box.append(&toc_scroll);

        let overlay = libadwaita::OverlaySplitView::builder()
            .max_sidebar_width(260.0)
            .sidebar(&toc_box)
            .content(&book_view.widget)
            .show_sidebar(false)
            .build();

        let toolbar_view = libadwaita::ToolbarView::builder().content(&overlay).build();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.add_bottom_bar(&bottom_bar);

        let toast_overlay = libadwaita::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("GutenReader")
            .default_width(1200)
            .default_height(800)
            .content(&toast_overlay)
            .build();

        let app_ref = Rc::new(Self {
            window: window.clone(),
            document: document.clone(),
            settings: settings.clone(),
            annotations: annotations.clone(),
            book_view: book_view.clone(),
            toc_entries: toc_entries.clone(),
            title_label: title_label.clone(),
            toc_list: toc_list.clone(),
            progress_label: progress_label.clone(),
            header_bar: header_bar.clone(),
            bottom_bar: bottom_bar.clone(),
            overlay: overlay.clone(),
            toc_btn: toc_btn.clone(),
            focus_btn: focus_btn.clone(),
            two_page_btn: two_page_btn.clone(),
            voice_btn: voice_btn.clone(),
            annotate_btn: annotate_btn.clone(),
            toast_overlay: toast_overlay.clone(),
            self_weak: RefCell::new(std::rc::Weak::new()),
            tts_engine: Rc::new(RefCell::new(Some(crate::tts::TtsEngine::new()))),
        });
        app_ref.self_weak.replace(Rc::downgrade(&app_ref));

        // ---- Inicializar menú de voces TTS ----
        app_ref.init_voice_menu();

        // ---- Actions ----
        let app_weak = Rc::downgrade(&app_ref);
        let action_day = gtk::gio::SimpleAction::new("theme_day", None);
        action_day.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.set_profile("day");
            }
        });
        app.add_action(&action_day);

        let app_weak = Rc::downgrade(&app_ref);
        let action_night = gtk::gio::SimpleAction::new("theme_night", None);
        action_night.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.set_profile("night");
            }
        });
        app.add_action(&action_night);

        let app_weak = Rc::downgrade(&app_ref);
        let action_sepia = gtk::gio::SimpleAction::new("theme_sepia", None);
        action_sepia.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.set_profile("sepia");
            }
        });
        app.add_action(&action_sepia);

        let app_weak = Rc::downgrade(&app_ref);
        let action_show_shortcuts = gtk::gio::SimpleAction::new("show-shortcuts", None);
        action_show_shortcuts.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.show_shortcuts_dialog();
            }
        });
        app_ref.add_action(&action_show_shortcuts);

        let app_weak = Rc::downgrade(&app_ref);
        let action_quit = gtk::gio::SimpleAction::new("quit", None);
        action_quit.connect_activate(move |_, _| {
            if let Some(app) = app_weak.upgrade() {
                app.window.close();
            }
        });
        app.add_action(&action_quit);
        app.set_accels_for_action("app.quit", &["<Control>q"]);

        // ---- Signals ----
        let overlay_weak = overlay.clone();
        toc_btn.connect_toggled(move |btn| {
            overlay_weak.set_show_sidebar(btn.is_active());
        });

        let app_weak = Rc::downgrade(&app_ref);
        open_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                gtk::glib::spawn_future_local(async move {
                    app.open_file_dialog().await;
                });
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        search_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                app.open_search_dialog();
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        tts_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                app.toggle_tts();
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        prev_page_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                let app2 = Rc::downgrade(&app);
                app.book_view.scroll_prev_page(move || {
                    if let Some(app) = app2.upgrade() {
                        app.prev_chapter();
                    }
                });
            }
        });
        let app_weak = Rc::downgrade(&app_ref);
        next_page_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                let app2 = Rc::downgrade(&app);
                app.book_view.scroll_next_page(move || {
                    if let Some(app) = app2.upgrade() {
                        app.next_chapter();
                    }
                });
            }
        });
        let app_weak = Rc::downgrade(&app_ref);
        prev_chap_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                app.prev_chapter();
            }
        });
        let app_weak = Rc::downgrade(&app_ref);
        next_chap_btn.connect_clicked(move |_| {
            if let Some(app) = app_weak.upgrade() {
                app.next_chapter();
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        focus_btn.connect_toggled(move |btn| {
            if let Some(app) = app_weak.upgrade() {
                app.set_focus_mode(btn.is_active());
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        two_page_btn.connect_toggled(move |btn| {
            if let Some(app) = app_weak.upgrade() {
                {
                    let mut s = app.settings.borrow_mut();
                    s.two_page_mode = btn.is_active();
                    let _ = s.save();
                }
                app.book_view.apply_settings(&*app.settings.borrow());
                app.load_current_chapter();
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        book_view.set_on_navigate(move |uri| {
            if let Some(app) = app_weak.upgrade() {
                app.handle_internal_link(&uri);
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        book_view.set_on_load_finished(move || {
            if let Some(app) = app_weak.upgrade() {
                app.apply_annotations_highlights();
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        book_view.set_on_key(move |keyval, state| {
            let key_name = keyval.name().map(|s| s.to_string()).unwrap_or_default();
            if let Some(app) = app_weak.upgrade() {
                let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                let is_alt = state.contains(gtk::gdk::ModifierType::ALT_MASK);

                let is_left = keyval == gtk::gdk::Key::Left || keyval == gtk::gdk::Key::Page_Up;
                let is_right = keyval == gtk::gdk::Key::Right
                    || keyval == gtk::gdk::Key::Page_Down
                    || keyval == gtk::gdk::Key::space;

                if is_left {
                    if is_ctrl || is_alt {
                        app.prev_chapter();
                    } else {
                        let app2 = Rc::downgrade(&app);
                        app.book_view.scroll_prev_page(move || {
                            if let Some(app) = app2.upgrade() {
                                app.prev_chapter();
                            }
                        });
                    }
                    return true;
                }
                if is_right {
                    if is_ctrl || is_alt {
                        app.next_chapter();
                    } else {
                        let app2 = Rc::downgrade(&app);
                        app.book_view.scroll_next_page(move || {
                            if let Some(app) = app2.upgrade() {
                                app.next_chapter();
                            }
                        });
                    }
                    return true;
                }
                if key_name == "o" && is_ctrl {
                    let app2 = Rc::downgrade(&app);
                    gtk::glib::spawn_future_local(async move {
                        if let Some(app) = app2.upgrade() {
                            app.open_file_dialog().await;
                        }
                    });
                    return true;
                }
                if key_name == "f" && is_ctrl {
                    app.open_search_dialog();
                    return true;
                }
                if key_name == "F5" {
                    app.toggle_tts();
                    return true;
                }
                if key_name == "F9" {
                    let active = !app.toc_btn.is_active();
                    app.toc_btn.set_active(active);
                    return true;
                }
                if key_name == "plus" || key_name == "equal" || key_name == "KP_Add" {
                    if let Ok(opt) = app.tts_engine.try_borrow() {
                        if let Some(ref engine) = *opt {
                            let new_speed = engine.increase_speed();
                            app.show_toast(&format!("Velocidad TTS: {:.1}x", new_speed));
                            let _ = engine.restart();
                        }
                    }
                    return true;
                }
                if key_name == "minus" || key_name == "KP_Subtract" {
                    if let Ok(opt) = app.tts_engine.try_borrow() {
                        if let Some(ref engine) = *opt {
                            let new_speed = engine.decrease_speed();
                            app.show_toast(&format!("Velocidad TTS: {:.1}x", new_speed));
                            let _ = engine.restart();
                        }
                    }
                    return true;
                }
                if key_name == "F11" {
                    let active = !app.focus_btn.is_active();
                    app.focus_btn.set_active(active);
                    return true;
                }
                if key_name == "a" && is_ctrl && state.contains(gtk::gdk::ModifierType::SHIFT_MASK)
                {
                    app.open_annotation_popover();
                    return true;
                }
            }
            false
        });

        let app_weak = Rc::downgrade(&app_ref);
        toc_list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let idx = row.index() as usize;
            if let Some(app) = app_weak.upgrade() {
                let hrefs = app.toc_entries.borrow();
                if let Some(href) = hrefs.get(idx) {
                    let href = href.clone();
                    drop(hrefs);
                    let mut navigated = false;
                    {
                        let mut doc_opt = app.document.borrow_mut();
                        if let Some(doc) = doc_opt.as_mut() {
                            navigated = doc.goto_toc_href(&href);
                        }
                    }
                    if navigated {
                        app.load_current_chapter();
                    } else {
                        eprintln!("[GutenReader] No se pudo navegar al href del TOC: {}", href);
                    }
                }
            }
        });

        let app_weak = Rc::downgrade(&app_ref);
        window.connect_close_request(move |_| {
            if let Some(app) = app_weak.upgrade() {
                let mut doc_opt = app.document.borrow_mut();
                if let Some(doc) = doc_opt.as_mut() {
                    let _ = doc.save_position(0.0);
                }
                // Detener TTS al cerrar la ventana
                if let Ok(opt) = app.tts_engine.try_borrow() {
                    if let Some(ref engine) = *opt {
                        let _ = engine.stop();
                    }
                }
            }
            gtk::glib::Propagation::Proceed
        });

        app_ref
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn add_action(&self, action: &gtk::gio::SimpleAction) {
        self.window.add_action(action);
    }

    pub fn open_epub(&self, path: &std::path::Path) {
        match DocumentModel::open(path) {
            Ok(mut doc) => {
                eprintln!("[GutenReader] EPUB abierto: {:?}", path);
                eprintln!("[GutenReader] Metadata: {:?}", doc.metadata());
                eprintln!("[GutenReader] Spine len: {}", doc.spine_len());
                if let Some(id) = doc.current_chapter_id() {
                    eprintln!("[GutenReader] Primer spine id: {}", id);
                    if let Ok(item) = doc.core.get_item(id) {
                        eprintln!("[GutenReader] Primer item media_type: {}", item.media_type);
                    }
                }

                // Ir al primer capítulo con contenido sustancial
                let first = doc.find_first_content_chapter();
                if first != doc.spine_index {
                    eprintln!(
                        "[GutenReader] Saltando de spine[{}] a spine[{}] (primer capítulo largo)",
                        doc.spine_index, first
                    );
                    doc.goto_spine_index(first);
                }

                let book_id = doc.book_id();
                let hash = doc
                    .core
                    .file_hash
                    .clone()
                    .unwrap_or_else(|| book_id.clone());
                *self.annotations.borrow_mut() = Some(AnnotationStore::new(&hash));
                *self.document.borrow_mut() = Some(doc);
                self.refresh_ui();
                self.load_current_chapter();
            }
            Err(e) => {
                self.show_toast(&format!("Error abriendo EPUB: {}", e));
            }
        }
    }

    fn set_profile(&self, key: &str) {
        {
            let mut s = self.settings.borrow_mut();
            s.current_profile = key.to_string();
            let _ = s.save();
        }
        self.book_view.apply_settings(&*self.settings.borrow());
        // Recargar capítulo para que aplique CSS
        self.load_current_chapter();
    }

    fn refresh_ui(&self) {
        self.populate_toc();
        if let Some(doc) = self.document.borrow().as_ref() {
            if let Some(meta) = doc.metadata() {
                self.title_label.set_label(&meta.title);
                self.window.set_title(Some(&meta.title));
            }
        }
        self.update_progress_label();
    }

    fn load_current_chapter(&self) {
        let mut doc_opt = self.document.borrow_mut();
        let Some(doc) = doc_opt.as_mut() else { return };
        let id = match doc.current_chapter_id() {
            Some(i) => i.to_string(),
            None => {
                self.show_toast("No hay capítulo para mostrar");
                return;
            }
        };
        let path = match doc.core.get_resource_path(&id) {
            Ok(p) => p,
            Err(e) => {
                self.show_toast(&format!("Error localizando capítulo: {}", e));
                return;
            }
        };
        let base_uri = doc.base_uri().unwrap_or_else(|| "about:blank".to_string());
        drop(doc_opt);

        let file_uri = gtk::glib::filename_to_uri(&path, None)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| "about:blank".to_string());

        eprintln!("[GutenReader] Cargando capítulo id={}", id);
        eprintln!("[GutenReader] file_uri={}", file_uri);
        eprintln!("[GutenReader] base_uri={}", base_uri);

        self.book_view.apply_settings(&*self.settings.borrow());
        self.book_view.load_chapter(&file_uri, &base_uri);
        self.update_progress_label();
    }

    fn next_chapter(&self) {
        {
            let mut doc_opt = self.document.borrow_mut();
            let Some(doc) = doc_opt.as_mut() else { return };
            let _ = doc.save_position(0.0);
            if !doc.goto_next() {
                self.show_toast("Fin del libro");
                return;
            }
        }
        self.load_current_chapter();
    }

    fn prev_chapter(&self) {
        {
            let mut doc_opt = self.document.borrow_mut();
            let Some(doc) = doc_opt.as_mut() else { return };
            let _ = doc.save_position(0.0);
            if !doc.goto_prev() {
                self.show_toast("Inicio del libro");
                return;
            }
        }
        self.load_current_chapter();
    }

    fn handle_internal_link(&self, uri: &str) {
        let path_part = uri.strip_prefix("file://").unwrap_or(uri);
        let mut doc_opt = self.document.borrow_mut();
        let Some(doc) = doc_opt.as_mut() else { return };

        if let Some(opf_dir) = doc.core.opf_dir.as_ref() {
            let opf_canon = opf_dir.canonicalize().unwrap_or_else(|_| opf_dir.clone());
            if let Ok(rel) = std::path::Path::new(path_part).strip_prefix(&opf_canon) {
                let href = rel.to_string_lossy().replace('\\', "/");
                if doc.goto_toc_href(&href) {
                    drop(doc_opt);
                    self.load_current_chapter();
                    return;
                }
            }
        }
        for (idx, id) in doc.core.spine.iter().enumerate() {
            if let Ok(item) = doc.core.get_item(id) {
                if path_part.ends_with(&item.href) || item.href == path_part {
                    doc.goto_spine_index(idx);
                    drop(doc_opt);
                    self.load_current_chapter();
                    return;
                }
            }
        }
    }

    fn populate_toc(&self) {
        while let Some(child) = self.toc_list.first_child() {
            self.toc_list.remove(&child);
        }
        let mut entries_guard = self.toc_entries.borrow_mut();
        entries_guard.clear();

        let doc_opt = self.document.borrow();
        let Some(doc) = doc_opt.as_ref() else { return };
        let toc = match doc.toc() {
            Ok(t) => t,
            Err(_) => return,
        };
        drop(doc_opt);

        for entry in &toc {
            entries_guard.push(entry.href.clone());
            let row = gtk::ListBoxRow::new();
            let label = gtk::Label::builder()
                .label(&entry.title)
                .xalign(0.0)
                .margin_start(8 * entry.level as i32)
                .margin_top(6)
                .margin_bottom(6)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            row.set_child(Some(&label));
            self.toc_list.append(&row);
        }
    }

    fn update_progress_label(&self) {
        let doc_opt = self.document.borrow();
        if let Some(doc) = doc_opt.as_ref() {
            let current = doc.spine_index + 1;
            let total = doc.spine_len();
            self.progress_label
                .set_text(&format!("{} / {}", current, total));
        } else {
            self.progress_label.set_text("- / -");
        }
    }

    fn set_focus_mode(&self, active: bool) {
        if active {
            self.header_bar.set_visible(false);
            self.bottom_bar.set_visible(false);
            self.overlay.set_show_sidebar(false);
        } else {
            self.header_bar.set_visible(true);
            self.bottom_bar.set_visible(true);
        }
    }

    async fn open_file_dialog(&self) {
        let dialog = gtk::FileDialog::builder()
            .title("Abrir EPUB")
            .modal(true)
            .build();
        let filter = gtk::FileFilter::new();
        filter.add_suffix("epub");
        filter.set_name(Some("EPUB"));
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));

        match dialog.open_future(Some(&self.window)).await {
            Ok(file) => {
                if let Some(path) = file.path() {
                    self.open_epub(&path);
                }
            }
            Err(_) => {}
        }
    }

    fn open_search_dialog(&self) {
        let dialog = crate::search::SearchDialog::new(&self.window);

        // Callback de búsqueda
        let doc_weak = Rc::downgrade(&self.document);
        dialog.set_on_search(move |query| {
            if let Some(doc_rc) = doc_weak.upgrade() {
                let doc_opt = doc_rc.borrow();
                if let Some(doc) = doc_opt.as_ref() {
                    return doc.search(query).unwrap_or_default();
                }
            }
            Vec::new()
        });

        // Callback al seleccionar resultado
        let app_weak = self.self_weak.borrow().clone();
        let dialog_weak = Rc::downgrade(&dialog);
        dialog.set_on_result_activated(move |idx| {
            if let Some(d) = dialog_weak.upgrade() {
                let res_opt = {
                    let results = d.results.borrow();
                    results.get(idx).cloned()
                };
                if let Some(res) = res_opt {
                    if let Some(app) = app_weak.upgrade() {
                        let mut doc_opt = app.document.borrow_mut();
                        if let Some(doc) = doc_opt.as_mut() {
                            doc.goto_chapter_id(&res.chapter_id);
                            drop(doc_opt);
                            app.load_current_chapter();
                            app.book_view.scroll_to_anchor(&res.block_id);
                        }
                    }
                }
                d.dialog.close();
            }
        });

        dialog.present();
    }

    fn toggle_tts(&self) {
        if let Some(ref engine) = *self.tts_engine.borrow() {
            if engine.is_speaking() {
                let _ = engine.stop();
                return;
            }
        }
        let tts_engine = self.tts_engine.clone();
        self.book_view.get_text_async(move |text| {
            let text = text.trim().to_string();
            if text.is_empty() {
                return;
            }
            if let Ok(opt) = tts_engine.try_borrow() {
                if let Some(ref engine) = *opt {
                    let _ = engine.speak(&text);
                }
            }
        });
    }

    fn show_toast(&self, message: &str) {
        let toast = libadwaita::Toast::new(message);
        self.toast_overlay.add_toast(toast);
    }

    fn init_voice_menu(&self) {
        if let Ok(opt) = self.tts_engine.try_borrow() {
            if let Some(ref engine) = *opt {
                let voices = engine.voices();
                if voices.is_empty() {
                    self.voice_btn.set_visible(false);
                    return;
                }

                let menu = gtk::gio::Menu::new();
                for (idx, voice) in voices.iter().enumerate() {
                    let action_name = format!("app.set_voice_{}", idx);
                    let label = format!("{} ({})", voice.name, voice.language);
                    menu.append(Some(&label), Some(&action_name));

                    // Crear action
                    let voice_name = voice.name.clone();
                    let app_weak = self.self_weak.borrow().clone();
                    let action = gtk::gio::SimpleAction::new(&format!("set_voice_{}", idx), None);
                    action.connect_activate(move |_, _| {
                        if let Some(app) = app_weak.upgrade() {
                            if let Ok(opt) = app.tts_engine.try_borrow() {
                                if let Some(ref engine) = *opt {
                                    engine.set_voice(&voice_name);
                                }
                            }
                            if let Ok(mut s) = app.settings.try_borrow_mut() {
                                s.tts_voice = voice_name.clone();
                                let _ = s.save();
                            }
                        }
                    });
                    self.window.add_action(&action);
                }
                self.voice_btn.set_menu_model(Some(&menu));

                // Restaurar voz guardada
                let saved_voice = self.settings.borrow().tts_voice.clone();
                if !saved_voice.is_empty() {
                    engine.set_voice(&saved_voice);
                }
            }
        }
    }

    fn show_shortcuts_dialog(&self) {
        let dialog = libadwaita::ShortcutsDialog::new();
        dialog.set_title("Atajos de teclado");
        dialog.set_content_width(600);
        dialog.set_content_height(500);

        let section = libadwaita::ShortcutsSection::new(Some("General"));

        section.add(libadwaita::ShortcutsItem::new("Abrir EPUB", "<Control>o"));
        section.add(libadwaita::ShortcutsItem::new("Buscar", "<Control>f"));
        section.add(libadwaita::ShortcutsItem::new("Tabla de contenidos", "F9"));
        section.add(libadwaita::ShortcutsItem::new("Página anterior", "Left"));
        section.add(libadwaita::ShortcutsItem::new("Página siguiente", "Right"));
        section.add(libadwaita::ShortcutsItem::new(
            "Capítulo anterior",
            "<Control>Left",
        ));
        section.add(libadwaita::ShortcutsItem::new(
            "Capítulo siguiente",
            "<Control>Right",
        ));
        section.add(libadwaita::ShortcutsItem::new(
            "Leer en voz alta (TTS)",
            "F5",
        ));
        section.add(libadwaita::ShortcutsItem::new(
            "Aumentar velocidad TTS",
            "plus",
        ));
        section.add(libadwaita::ShortcutsItem::new(
            "Disminuir velocidad TTS",
            "minus",
        ));
        section.add(libadwaita::ShortcutsItem::new(
            "Anotar selección",
            "<Control><Shift>a",
        ));
        section.add(libadwaita::ShortcutsItem::new("Modo foco", "F11"));
        section.add(libadwaita::ShortcutsItem::new("Salir", "<Control>q"));

        dialog.add(section);
        dialog.present(Some(&self.window));
    }

    fn apply_annotations_highlights(&self) {
        let annotations_opt = self.annotations.borrow();
        let Some(store) = annotations_opt.as_ref() else {
            return;
        };
        let doc_opt = self.document.borrow();
        let Some(doc) = doc_opt.as_ref() else { return };
        let chapter_id = match doc.current_chapter_id() {
            Some(id) => id,
            None => return,
        };
        let anns: Vec<(String, String, Option<String>)> = store
            .for_chapter(&chapter_id)
            .into_iter()
            .map(|a| (a.selected_text.clone(), a.color.clone(), a.anchor.clone()))
            .collect();
        drop(doc_opt);
        drop(annotations_opt);
        if !anns.is_empty() {
            self.book_view.apply_highlights(&anns);
        }
    }

    fn open_annotation_popover(&self) {
        let app_weak = self.self_weak.borrow().clone();
        let popover = crate::annotation_dialog::AnnotationPopover::new(&self.annotate_btn);
        popover.clear_note();

        let popover_weak = Rc::downgrade(&popover);
        popover.set_on_save(move |color, note| {
            let _ = popover_weak.upgrade();
            let app2 = app_weak.clone();
            if let Some(app) = app_weak.upgrade() {
                // Obtener selección del WebView
                app.book_view
                    .get_selection_async(move |selected_text, anchor| {
                        if let Some(app) = app2.upgrade() {
                            if selected_text.is_empty() {
                                app.show_toast("Selecciona texto para anotar");
                                return;
                            }
                            let doc_opt = app.document.borrow();
                            let Some(doc) = doc_opt.as_ref() else {
                                app.show_toast("Abre un libro primero");
                                return;
                            };
                            let chapter_id = match doc.current_chapter_id() {
                                Some(id) => id,
                                None => {
                                    app.show_toast("No se pudo identificar el capítulo");
                                    return;
                                }
                            };
                            let chapter_id = chapter_id.to_string();
                            drop(doc_opt);

                            let created_at = gtk::glib::DateTime::now_local()
                                .map(|dt| {
                                    dt.format("%x %X")
                                        .map(|s| s.to_string())
                                        .unwrap_or_default()
                                })
                                .unwrap_or_default();
                            let ann_color = color.clone();
                            let ann_anchor = anchor.clone();
                            let annotation = crate::annotations::Annotation {
                                chapter_id,
                                selected_text: selected_text.clone(),
                                note,
                                color,
                                created_at,
                                anchor,
                            };

                            if let Ok(mut ann_opt) = app.annotations.try_borrow_mut() {
                                if let Some(store) = ann_opt.as_mut() {
                                    store.add(annotation);
                                    let _ = store.save();
                                }
                            }

                            app.book_view.highlight_text(
                                &selected_text,
                                &ann_color,
                                ann_anchor.as_deref(),
                            );
                            app.show_toast("Anotación guardada");
                        }
                    });
            }
        });

        popover.popover.popup();
    }
}
