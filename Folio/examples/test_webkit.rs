use gtk::prelude::*;
use webkit6::prelude::*;
use webkit6::{UserContentInjectedFrames, UserStyleLevel, UserStyleSheet, WebView};

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.test.webkit")
        .build();

    app.connect_activate(|app| {
        let path = std::env::args()
            .nth(1)
            .expect("Uso: test_webkit <ruta al .epub>");

        let mut core = gutencore::GutenCore::open_epub(&path).expect("open_epub");

        // Buscar primer capítulo con >1500 chars de texto
        let mut target_id = None;
        for id in &core.spine {
            if let Ok(p) = core.get_resource_path(id) {
                if let Ok(html) = std::fs::read_to_string(&p) {
                    let text: String = html.chars().filter(|&c| c != '<' && c != '>').collect();
                    if text.len() > 1500 {
                        target_id = Some(id.clone());
                        break;
                    }
                }
            }
        }
        let id = target_id.unwrap_or_else(|| core.spine[0].clone());
        eprintln!("Capítulo seleccionado: {}", id);

        let path = core.get_resource_path(&id).unwrap();
        let html = std::fs::read_to_string(&path).unwrap();
        let parent = path.parent().unwrap();
        let canon = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        let base_uri = gtk::glib::filename_to_uri(canon, None).unwrap().to_string();
        let base_uri = if base_uri.ends_with('/') {
            base_uri
        } else {
            base_uri + "/"
        };

        let mut s = html;
        if s.starts_with("<?xml") {
            if let Some(end) = s.find("?>") {
                s.replace_range(..end + 2, "");
            }
        }
        if let Some(start) = s.find("<!DOCTYPE") {
            if let Some(end) = s[start..].find('>') {
                s.replace_range(start..start + end + 1, "");
            }
        }
        let clean = s.trim_start().to_string();

        let with_base = if clean.contains("<base") {
            clean
        } else if let Some(head_end) = clean.find("</head>") {
            let base_tag = format!(r#"<base href="{}"/>"#, base_uri);
            let mut s = clean;
            s.insert_str(head_end, &base_tag);
            s
        } else {
            clean
        };

        let wv = WebView::new();
        wv.set_vexpand(true);
        wv.set_hexpand(true);

        let css = r#"
            body {
                font-family: Georgia, serif !important;
                font-size: 14pt !important;
                line-height: 1.6 !important;
                margin: 2em !important;
                background-color: #faf8f5 !important;
                color: #1a1a1a !important;
            }
            img { max-width: 100%; height: auto; }
        "#;
        let ucm = wv.user_content_manager().unwrap();
        let sheet = UserStyleSheet::new(
            css,
            UserContentInjectedFrames::AllFrames,
            UserStyleLevel::User,
            &[],
            &[],
        );
        ucm.add_style_sheet(&sheet);

        wv.load_html(&with_base, Some(&base_uri));

        wv.connect_load_changed(|wv, event| {
            eprintln!("Load event: {:?}", event);
            if event == webkit6::LoadEvent::Finished {
                eprintln!("URI: {:?}", wv.uri());
                eprintln!("Title: {:?}", wv.title());
            }
        });

        wv.connect_load_failed(|_, event, uri, err| {
            eprintln!(
                "LOAD FAILED: event={:?} uri={} err={}",
                event,
                uri,
                err.message()
            );
            true
        });

        let win = gtk::ApplicationWindow::builder()
            .application(app)
            .title("WebKit Test")
            .default_width(800)
            .default_height(600)
            .child(&wv)
            .build();
        win.present();
    });

    app.run();
}
