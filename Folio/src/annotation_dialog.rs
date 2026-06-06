use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const ANNOTATION_COLORS: &[&str] = &[
    "#ffeb3b", // amarillo
    "#c8e6c9", // verde
    "#bbdefb", // azul
    "#f8bbd0", // rosa
    "#ffe0b2", // naranja
    "#e1bee7", // lila
    "#b2ebf2", // cyan
    "#ffcdd2", // rojo
    "#f0f4c3", // lima
    "#cfd8dc", // gris
];

pub struct AnnotationPopover {
    pub popover: gtk::Popover,
    selected_color: Rc<RefCell<String>>,
    note_view: gtk::TextView,
    on_save: Rc<RefCell<Option<Box<dyn Fn(String, String)>>>>,
}

impl AnnotationPopover {
    pub fn new(parent: &impl IsA<gtk::Widget>) -> Rc<Self> {
        let popover = gtk::Popover::new();
        popover.set_parent(parent.upcast_ref());
        popover.set_autohide(true);

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .width_request(320)
            .build();

        let title = gtk::Label::builder()
            .label("Añadir anotación")
            .css_classes(vec!["heading".to_string()])
            .build();
        vbox.append(&title);

        let color_label = gtk::Label::builder().label("Color").xalign(0.0).build();
        vbox.append(&color_label);

        let color_box = gtk::FlowBox::builder()
            .homogeneous(true)
            .row_spacing(6)
            .column_spacing(6)
            .max_children_per_line(5)
            .build();

        let selected_color = Rc::new(RefCell::new(ANNOTATION_COLORS[0].to_string()));

        for (idx, color) in ANNOTATION_COLORS.iter().enumerate() {
            let btn = gtk::Button::builder()
                .width_request(32)
                .height_request(32)
                .css_classes(vec!["circular".to_string()])
                .build();
            let name = format!("anno-color-{}", idx);
            btn.set_widget_name(&name);
            let css = gtk::CssProvider::new();
            css.load_from_string(&format!(
                "button#{} {{ background-color: {}; border: 2px solid transparent; }}\n\
                 button#{}:hover {{ border-color: @accent_bg_color; }}\n\
                 button#{}.selected {{ border-color: @theme_fg_color; }}",
                name, color, name, name
            ));
            if let Some(display) = gtk::gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &css,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }

            let sel_color = selected_color.clone();
            btn.connect_clicked(move |_| {
                *sel_color.borrow_mut() = color.to_string();
            });
            color_box.append(&btn);
        }
        vbox.append(&color_box);

        let note_label = gtk::Label::builder().label("Nota").xalign(0.0).build();
        vbox.append(&note_label);

        let note_view = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .height_request(80)
            .build();
        let note_scroll = gtk::ScrolledWindow::builder()
            .child(&note_view)
            .has_frame(true)
            .build();
        vbox.append(&note_scroll);

        let save_btn = gtk::Button::builder()
            .label("Guardar")
            .css_classes(vec!["suggested-action".to_string()])
            .halign(gtk::Align::End)
            .build();
        vbox.append(&save_btn);

        popover.set_child(Some(&vbox));

        let on_save: Rc<RefCell<Option<Box<dyn Fn(String, String)>>>> = Rc::new(RefCell::new(None));

        let sel_color = selected_color.clone();
        let note_view_ref = note_view.clone();
        let on_save_ref = on_save.clone();
        let popover_ref = popover.clone();
        save_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *on_save_ref.borrow() {
                let buffer = note_view_ref.buffer();
                let note = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                let color = sel_color.borrow().clone();
                cb(color, note.to_string());
            }
            popover_ref.popdown();
        });

        Rc::new(Self {
            popover,
            selected_color,
            note_view,
            on_save,
        })
    }

    pub fn set_on_save<F: Fn(String, String) + 'static>(&self, f: F) {
        *self.on_save.borrow_mut() = Some(Box::new(f));
    }

    pub fn clear_note(&self) {
        let buffer = self.note_view.buffer();
        buffer.set_text("");
    }

    pub fn set_default_color(&self, color: &str) {
        *self.selected_color.borrow_mut() = color.to_string();
    }
}
