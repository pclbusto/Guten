mod annotation_dialog;
mod annotations;
mod app;
mod document;
mod search;
mod settings;
mod tts;
mod view;

use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static READER: RefCell<Option<Rc<app::GutenReaderApp>>> = RefCell::new(None);
}

fn main() {
    let app = libadwaita::Application::builder()
        .application_id("com.gutenreader.GutenReader")
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        let reader = app::GutenReaderApp::new(app);
        reader.present();
        READER.with(|r| *r.borrow_mut() = Some(reader));
    });

    app.connect_open(|app, files, _hint| {
        let reader = app::GutenReaderApp::new(app);
        reader.present();
        READER.with(|r| *r.borrow_mut() = Some(reader.clone()));
        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                reader.open_epub(&path);
            }
        }
    });

    app.run();
}
