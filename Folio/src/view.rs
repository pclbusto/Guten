use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use webkit6::prelude::*;
use webkit6::{
    NavigationPolicyDecision, PolicyDecisionType, UserContentInjectedFrames, UserStyleLevel,
    UserStyleSheet, WebView,
};

use crate::settings::ReaderSettings;

type NavigateCallback = Box<dyn Fn(String)>;

type KeyCallback = Box<dyn Fn(gtk::gdk::Key, gtk::gdk::ModifierType) -> bool>;

type LoadFinishedCallback = Box<dyn Fn()>;

pub struct BookView {
    pub widget: gtk::Box,
    webview: WebView,
    on_navigate: Rc<RefCell<Option<NavigateCallback>>>,
    on_key: Rc<RefCell<Option<KeyCallback>>>,
    on_load_finished: Rc<RefCell<Option<LoadFinishedCallback>>>,
    current_base_uri: Rc<RefCell<String>>,
}

impl BookView {
    pub fn new() -> Self {
        let webview = WebView::new();
        webview.set_vexpand(true);
        webview.set_hexpand(true);
        webview.set_size_request(400, 300);

        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        widget.append(&webview);

        let on_navigate: Rc<RefCell<Option<NavigateCallback>>> = Rc::new(RefCell::new(None));
        let on_key: Rc<RefCell<Option<KeyCallback>>> = Rc::new(RefCell::new(None));
        let on_load_finished: Rc<RefCell<Option<LoadFinishedCallback>>> =
            Rc::new(RefCell::new(None));
        let current_base_uri = Rc::new(RefCell::new(String::new()));

        let on_load_clone = on_load_finished.clone();
        webview.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished {
                if let Some(ref cb) = *on_load_clone.borrow() {
                    cb();
                }
            }
        });

        // EventControllerKey en el WebView con fase Capture para interceptar antes de WebKit
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let on_key_clone = on_key.clone();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            if let Some(ref cb) = *on_key_clone.borrow() {
                if cb(keyval, state) {
                    return gtk::glib::Propagation::Stop;
                }
            }
            gtk::glib::Propagation::Proceed
        });
        webview.add_controller(key_controller);

        // Interceptar clicks a links internos
        let cb_nav = on_navigate.clone();
        let base_uri_clone = current_base_uri.clone();
        webview.connect_decide_policy(move |_wv, decision, decision_type| {
            if decision_type == PolicyDecisionType::NavigationAction {
                if let Some(nav_decision) = decision.downcast_ref::<NavigationPolicyDecision>() {
                    if let Some(action) = nav_decision.navigation_action() {
                        if let Some(req) = action.request() {
                            if let Some(uri) = req.uri() {
                                let uri_str = uri.to_string();
                                let base = base_uri_clone.borrow();
                                if uri_str.starts_with("file://") && !uri_str.starts_with(&*base) {
                                    if let Some(ref cb) = *cb_nav.borrow() {
                                        cb(uri_str);
                                    }
                                    decision.ignore();
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            false
        });

        Self {
            widget,
            webview,
            on_navigate,
            on_key,
            on_load_finished,
            current_base_uri,
        }
    }

    pub fn set_on_load_finished<F: Fn() + 'static>(&self, f: F) {
        *self.on_load_finished.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_on_navigate<F: Fn(String) + 'static>(&self, f: F) {
        *self.on_navigate.borrow_mut() = Some(Box::new(f));
    }

    pub fn set_on_key<F: Fn(gtk::gdk::Key, gtk::gdk::ModifierType) -> bool + 'static>(&self, f: F) {
        *self.on_key.borrow_mut() = Some(Box::new(f));
    }

    pub fn load_chapter(&self, file_uri: &str, base_uri: &str) {
        *self.current_base_uri.borrow_mut() = base_uri.to_string();
        eprintln!("[BookView] load_uri={}", file_uri);
        self.webview.load_uri(file_uri);
        self.webview.grab_focus();
    }

    pub fn apply_settings(&self, settings: &ReaderSettings) {
        let ucm = self.webview.user_content_manager().unwrap();
        ucm.remove_all_style_sheets();

        let css = settings.generate_reader_css();
        let sheet = UserStyleSheet::new(
            &css,
            UserContentInjectedFrames::AllFrames,
            UserStyleLevel::User,
            &[],
            &[],
        );
        ucm.add_style_sheet(&sheet);
    }

    /// Scrollea una página hacia adelante. Si ya estaba al final, llama `on_end`.
    pub fn scroll_next_page<F: Fn() + 'static>(&self, on_end: F) {
        let script = r#"
            (function() {
                const root = document.documentElement;
                const maxX = Math.max(0, root.scrollWidth - window.innerWidth);
                const maxY = Math.max(0, root.scrollHeight - window.innerHeight);
                const horizontal = maxX > 5;
                const atEnd = horizontal
                    ? window.scrollX >= maxX - 5
                    : window.scrollY >= maxY - 5;

                if (atEnd) {
                    return true;
                }

                if (horizontal) {
                    window.scrollBy({left: window.innerWidth * 0.9, top: 0, behavior: 'smooth'});
                } else {
                    window.scrollBy({left: 0, top: window.innerHeight * 0.9, behavior: 'smooth'});
                }
                return false;
            })()
        "#;
        let wv = self.webview.clone();
        gtk::glib::spawn_future_local(async move {
            match wv.evaluate_javascript_future(script, None, None).await {
                Ok(value) => {
                    if value.to_boolean() {
                        on_end();
                    }
                }
                Err(_) => {}
            }
        });
    }

    /// Scrollea una página hacia atrás. Si ya estaba al inicio, llama `on_start`.
    pub fn scroll_prev_page<F: Fn() + 'static>(&self, on_start: F) {
        let script = r#"
            (function() {
                const root = document.documentElement;
                const maxX = Math.max(0, root.scrollWidth - window.innerWidth);
                const horizontal = maxX > 5;
                const atStart = horizontal ? window.scrollX <= 5 : window.scrollY <= 5;

                if (atStart) {
                    return true;
                }

                if (horizontal) {
                    window.scrollBy({left: -window.innerWidth * 0.9, top: 0, behavior: 'smooth'});
                } else {
                    window.scrollBy({left: 0, top: -window.innerHeight * 0.9, behavior: 'smooth'});
                }
                return false;
            })()
        "#;
        let wv = self.webview.clone();
        gtk::glib::spawn_future_local(async move {
            match wv.evaluate_javascript_future(script, None, None).await {
                Ok(value) => {
                    if value.to_boolean() {
                        on_start();
                    }
                }
                Err(_) => {}
            }
        });
    }

    pub fn scroll_to_anchor(&self, anchor: &str) {
        let script = format!(
            "document.getElementById('{}')?.scrollIntoView({{behavior:'smooth', block:'start'}});",
            anchor
        );
        let _ = self.webview.evaluate_javascript_future(&script, None, None);
    }

    pub fn get_text_async<F: FnOnce(String) + 'static>(&self, callback: F) {
        let script = r#"
            (function() {
                const limit = 1400;
                let out = "";
                const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
                while (walker.nextNode()) {
                    const node = walker.currentNode;
                    const parent = node.parentElement;
                    if (!parent || getComputedStyle(parent).display === "none") continue;
                    out += node.nodeValue;
                    if (out.length >= limit) return out.slice(0, limit);
                }
                return out;
            })()
        "#;
        let wv = self.webview.clone();
        gtk::glib::spawn_future_local(async move {
            match wv.evaluate_javascript_future(script, None, None).await {
                Ok(value) => {
                    let text = value.to_str().to_string();
                    callback(text);
                }
                Err(_) => {}
            }
        });
    }

    /// Obtiene la selección de texto actual y el anchor del elemento padre.
    pub fn get_selection_async<F: FnOnce(String, Option<String>) + 'static>(&self, callback: F) {
        let script = r#"
            (function() {
                var sel = window.getSelection();
                if (sel.rangeCount === 0) return JSON.stringify({text:"", anchor:null});
                var range = sel.getRangeAt(0);
                var text = range.toString().trim();
                var node = range.startContainer;
                while (node && node.nodeType !== 1) { node = node.parentNode; }
                var anchor = node ? (node.id || null) : null;
                return JSON.stringify({text: text, anchor: anchor});
            })()
        "#;
        let wv = self.webview.clone();
        gtk::glib::spawn_future_local(async move {
            match wv.evaluate_javascript_future(script, None, None).await {
                Ok(value) => {
                    let json = value.to_str().to_string();
                    let text = String::new();
                    let anchor: Option<String> = None;
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&json) {
                        let t = obj
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let a = obj
                            .get("anchor")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        callback(t, a);
                    } else {
                        callback(text, anchor);
                    }
                }
                Err(_) => callback(String::new(), None),
            }
        });
    }

    /// Resalta un texto específico con un color en el WebView.
    pub fn highlight_text(&self, text: &str, color: &str, anchor: Option<&str>) {
        let escaped_text = text
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        let anchor_part = anchor
            .map(|a| format!("'{}'", a.replace('\\', "\\\\").replace('"', "\\\"")))
            .unwrap_or_else(|| "null".to_string());
        let script = format!(
            r#"
            (function() {{
                var container = {anchor} ? document.getElementById({anchor}) : document.body;
                if (!container) container = document.body;
                var walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, null, false);
                var node;
                while (node = walker.nextNode()) {{
                    var idx = node.textContent.indexOf("{text}");
                    if (idx !== -1) {{
                        var range = document.createRange();
                        range.setStart(node, idx);
                        range.setEnd(node, idx + {len});
                        var span = document.createElement('span');
                        span.style.backgroundColor = '{color}';
                        span.style.borderRadius = '3px';
                        span.style.padding = '1px 0px';
                        span.className = 'gutenreader-highlight';
                        try {{
                            range.surroundContents(span);
                            return true;
                        }} catch(e) {{}}
                    }}
                }}
                return false;
            }})()
            "#,
            anchor = anchor_part,
            text = escaped_text,
            len = text.chars().count(),
            color = color.replace('\\', "\\\\").replace('"', "\\\""),
        );
        let _ = self.webview.evaluate_javascript_future(&script, None, None);
    }

    /// Aplica múltiples resaltados de anotaciones.
    pub fn apply_highlights(&self, annotations: &[(String, String, Option<String>)]) {
        for (text, color, anchor) in annotations {
            self.highlight_text(text, color, anchor.as_deref());
        }
    }

    /// Limpia todos los resaltados gutenreader del documento.
    pub fn clear_highlights(&self) {
        let script = r#"
            (function() {
                var spans = document.querySelectorAll('span.gutenreader-highlight');
                spans.forEach(function(span) {
                    var parent = span.parentNode;
                    while (span.firstChild) {
                        parent.insertBefore(span.firstChild, span);
                    }
                    parent.removeChild(span);
                    parent.normalize();
                });
            })()
        "#;
        let _ = self.webview.evaluate_javascript_future(script, None, None);
    }
}
