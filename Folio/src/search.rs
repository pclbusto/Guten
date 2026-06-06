use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SearchDialog {
    pub dialog: gtk::Window,
    entry: gtk::SearchEntry,
    list_box: gtk::ListBox,
    pub results: Rc<RefCell<Vec<gutencore::SearchResult>>>,
}

impl SearchDialog {
    pub fn new(parent: &impl IsA<gtk::Window>) -> Rc<Self> {
        let dialog = gtk::Window::builder()
            .transient_for(parent)
            .modal(true)
            .title("Buscar en el libro")
            .default_width(500)
            .default_height(400)
            .build();

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let entry = gtk::SearchEntry::builder()
            .placeholder_text("Buscar texto...")
            .hexpand(true)
            .build();

        let list_box = gtk::ListBox::builder()
            .vexpand(true)
            .hexpand(true)
            .css_classes(vec!["navigation-sidebar".to_string()])
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list_box)
            .vexpand(true)
            .build();

        vbox.append(&entry);
        vbox.append(&scroll);
        dialog.set_child(Some(&vbox));

        let results: Rc<RefCell<Vec<gutencore::SearchResult>>> = Rc::new(RefCell::new(Vec::new()));

        let s = Rc::new(Self {
            dialog: dialog.clone(),
            entry: entry.clone(),
            list_box: list_box.clone(),
            results: results.clone(),
        });

        s
    }

    pub fn set_on_search<F: Fn(&str) -> Vec<gutencore::SearchResult> + 'static>(&self, f: F) {
        let list = self.list_box.clone();
        let res_store = self.results.clone();
        let entry = self.entry.clone();
        entry.connect_activate(move |e| {
            let text = e.text();
            if text.len() < 2 {
                return;
            }
            let results = f(&text);
            *res_store.borrow_mut() = results.clone();
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            for r in results {
                let row = gtk::ListBoxRow::new();
                let label = gtk::Label::builder()
                    .label(&r.snippet)
                    .xalign(0.0)
                    .margin_top(6)
                    .margin_bottom(6)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .wrap(true)
                    .build();
                row.set_child(Some(&label));
                list.append(&row);
            }
        });
    }

    pub fn set_on_result_activated<F: Fn(usize) + 'static>(&self, f: F) {
        self.list_box.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            f(idx);
        });
    }

    pub fn present(&self) {
        self.dialog.present();
        self.entry.grab_focus();
    }
}
