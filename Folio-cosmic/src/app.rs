use cosmic::app::{Core, Task};
use cosmic::iced::event::{self, Event, Status as EventStatus};
use cosmic::iced::keyboard::{self, Key};
use cosmic::iced::{Color, Length, Subscription};
use cosmic::widget::{self, button};
use cosmic::{Application, ApplicationExt, Element};

use crate::annotations::AnnotationStore;
use crate::document::DocumentModel;
use crate::settings::{DEFAULT_READER_FONT_SIZE_PT, ReaderSettings};
use crate::tts::TtsEngine;
use folio::content::{ContentBlock, StyleMap, StyledSpan};
use folio::css::{self, CssRule, FontFaceRule};
use folio::fonts::{self, EpubFont, FontNameMap};
use folio::image_resources::ImageMetadataCache;
use folio::reader::renderer::{extract_heading, parse_hex_color};
use folio::reader::text_canvas::{ReaderLayoutCache, ReaderMetrics};

use gutencore::TocEntry;
use rfd::AsyncFileDialog;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SidebarTab {
    Toc,
    Search,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum Message {
    OpenFile,
    FileSelected(Option<PathBuf>),
    CloseBook,

    NextChapter,
    PrevChapter,

    ToggleSidebar,
    OpenSearchSidebar,
    SelectSidebarTab(SidebarTab),
    SelectTocEntry(usize),

    SearchQueryChanged(String),
    ExecuteSearch,
    SelectSearchResult(usize),

    ToggleReadingPanel,
    SetProfile(String),
    FontIncrease,
    FontDecrease,
    FontReset,
    SelectFont(String),
    LineHeightIncrease,
    LineHeightDecrease,
    MarginIncrease,
    MarginDecrease,

    ToggleMenu,
    ToggleTts,

    ImageClicked(String),
    CloseImage,

    SetReaderScroll(f32),
    PageDown,
    PageUp,
    KeyPressed(keyboard::Event),

    BookOpened(Result<OpenedBook, String>),
    Visore(visore::viewer::ViewerMessage),
}

#[derive(Debug, Clone)]
pub(crate) struct OpenedBook {
    path: PathBuf,
    doc: Arc<std::sync::Mutex<Option<DocumentModel>>>,
    toc: Vec<TocEntry>,
    annotations: AnnotationStore,
    spine_len: usize,
    spine_idx: usize,
}

pub(crate) struct App {
    core: Core,
    document: Option<DocumentModel>,
    settings: ReaderSettings,
    annotations: Option<AnnotationStore>,
    toc_entries: Vec<TocEntry>,
    current_blocks: Vec<ContentBlock>,
    chapter_title: String,
    sidebar_open: bool,
    sidebar_tab: SidebarTab,
    search_query: String,
    search_results: Vec<gutencore::SearchResult>,
    status: Option<String>,
    spine_progress: (usize, usize),
    tts_engine: TtsEngine,
    scroll_id: widget::Id,
    showing_image: bool,
    visore_state: visore::ViewerState,
    style_map: StyleMap,
    epub_fonts: Vec<EpubFont>,
    css_rules: Vec<CssRule>,
    font_faces: Vec<(FontFaceRule, std::path::PathBuf)>,
    font_name_map: FontNameMap,
    reading_panel_open: bool,
    menu_open: bool,
    reader_scroll_y: f32,
    reader_metrics: Rc<RefCell<ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<ImageMetadataCache>>,
    reader_layout_cache: Rc<RefCell<ReaderLayoutCache>>,
}

fn perform_async<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
    f: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    cosmic::task::future(async move { f(future.await) })
}

fn recent_shortcut_index(value: &str) -> Option<usize> {
    let digit = value.chars().next()?.to_digit(10)? as usize;
    if value.chars().count() == 1 && (1..=9).contains(&digit) {
        Some(digit - 1)
    } else {
        None
    }
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = Option<PathBuf>;
    type Message = Message;
    const APP_ID: &'static str = "com.gutenreader.GutenReaderCosmic";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            button::standard("\u{2630}")
                .on_press(Message::ToggleSidebar)
                .into(),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut items: Vec<Element<'_, Self::Message>> = Vec::new();
        if self.document.is_some() {
            items.push(
                button::standard("Abrir otro libro (Ctrl+O)")
                    .on_press(Message::OpenFile)
                    .into(),
            );
            items.push(
                button::standard("\u{1f50d}")
                    .on_press(Message::OpenSearchSidebar)
                    .into(),
            );
            items.push(
                button::standard("Aa")
                    .on_press(Message::ToggleReadingPanel)
                    .into(),
            );
            items.push(
                button::standard("\u{22ef}")
                    .on_press(Message::ToggleMenu)
                    .into(),
            );
        }
        items
    }

    fn init(core: Core, epub_path: Self::Flags) -> (Self, Task<Self::Message>) {
        let settings = ReaderSettings::load();
        let tts_engine = TtsEngine::new();

        let mut app = Self {
            core,
            document: None,
            settings,
            annotations: None,
            toc_entries: Vec::new(),
            current_blocks: Vec::new(),
            chapter_title: String::new(),
            sidebar_open: false,
            sidebar_tab: SidebarTab::Toc,
            search_query: String::new(),
            search_results: Vec::new(),
            status: None,
            spine_progress: (0, 0),
            tts_engine,
            scroll_id: widget::Id::unique(),
            showing_image: false,
            visore_state: visore::ViewerState::default(),
            style_map: StyleMap::default(),
            epub_fonts: Vec::new(),
            css_rules: Vec::new(),
            font_faces: Vec::new(),
            font_name_map: FontNameMap::default(),
            reading_panel_open: false,
            menu_open: false,
            reader_scroll_y: 0.0,
            reader_metrics: Rc::new(RefCell::new(ReaderMetrics::default())),
            image_metadata_cache: Rc::new(RefCell::new(ImageMetadataCache::default())),
            reader_layout_cache: Rc::new(RefCell::new(ReaderLayoutCache::default())),
        };
        app.update_style_map_sizes();
        app.set_header_title("Folio".into());
        let task = epub_path.map_or_else(Task::none, |path| {
            Task::done(cosmic::Action::App(Message::FileSelected(Some(path))))
        });
        (app, task)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::OpenFile => {
                return perform_async(
                    async {
                        AsyncFileDialog::new()
                            .add_filter("EPUB", &["epub"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::FileSelected,
                );
            }

            Message::FileSelected(Some(path)) => {
                self.sidebar_open = false;
                self.reading_panel_open = false;
                self.menu_open = false;
                self.showing_image = false;
                self.status = Some("Abriendo...".into());

                return perform_async(
                    async move {
                        match DocumentModel::open(&path) {
                            Ok(mut doc) => {
                                let first = doc.find_first_content_chapter();
                                doc.goto_spine_index(first);
                                let toc = doc.toc().unwrap_or_default();
                                let book_id = doc.book_id();
                                let hash = doc
                                    .core
                                    .file_hash
                                    .clone()
                                    .unwrap_or_else(|| book_id.clone());
                                let annotations = AnnotationStore::new(&hash);
                                let spine_len = doc.spine_len();
                                let spine_idx = doc.spine_index;
                                Ok(OpenedBook {
                                    path,
                                    doc: Arc::new(std::sync::Mutex::new(Some(doc))),
                                    toc,
                                    annotations,
                                    spine_len,
                                    spine_idx,
                                })
                            }
                            Err(e) => Err(format!("Error: {}", e)),
                        }
                    },
                    Message::BookOpened,
                );
            }

            Message::FileSelected(None) => {}

            Message::BookOpened(Ok(book)) => {
                self.image_metadata_cache.borrow_mut().clear();
                self.settings.remember_recent_book(&book.path);
                let _ = self.settings.save();
                let mut guard = book.doc.lock().unwrap();
                let doc = guard.take();
                if let Some(ref doc) = doc {
                    self.epub_fonts = fonts::extract_epub_fonts(&doc.core.manifest, |id| {
                        doc.core.get_resource_path(id)
                    });
                    let (rules, faces) = self.load_epub_css(doc);
                    self.font_name_map = fonts::build_font_name_map(&faces, &self.epub_fonts);
                    self.css_rules = rules;
                    self.font_faces = faces;
                }
                self.document = doc;
                self.annotations = Some(book.annotations);
                self.toc_entries = book.toc;
                self.spine_progress = (book.spine_idx + 1, book.spine_len);
                self.status = None;
                self.reset_reader_scroll();
                self.load_chapter_sync();
            }
            Message::BookOpened(Err(e)) => {
                self.status = Some(e);
            }

            Message::CloseBook => {
                if let Some(ref doc) = self.document {
                    let _ = doc.save_position(0.0);
                }
                let _ = self.tts_engine.stop();
                self.document = None;
                self.annotations = None;
                self.toc_entries.clear();
                self.current_blocks.clear();
                self.chapter_title.clear();
                self.spine_progress = (0, 0);
                self.search_results.clear();
                self.search_query.clear();
                self.sidebar_open = false;
                self.reading_panel_open = false;
                self.menu_open = false;
                self.showing_image = false;
                self.status = None;
                self.epub_fonts.clear();
                self.css_rules.clear();
                self.font_faces.clear();
                self.font_name_map = FontNameMap::default();
                self.image_metadata_cache.borrow_mut().clear();
                self.invalidate_reader_layout();
                self.set_header_title("Folio".into());
            }

            Message::NextChapter => {
                if let Some(ref mut doc) = self.document {
                    let _ = doc.save_position(0.0);
                    if doc.goto_next() {
                        self.spine_progress = (doc.spine_index + 1, doc.spine_len());
                    } else {
                        self.status = Some("Fin del libro".into());
                        return Task::none();
                    }
                }
                self.showing_image = false;
                self.reset_reader_scroll();
                self.load_chapter_sync();
            }
            Message::PrevChapter => {
                if let Some(ref mut doc) = self.document {
                    let _ = doc.save_position(0.0);
                    if doc.goto_prev() {
                        self.spine_progress = (doc.spine_index + 1, doc.spine_len());
                    } else {
                        self.status = Some("Inicio del libro".into());
                        return Task::none();
                    }
                }
                self.showing_image = false;
                self.reset_reader_scroll();
                self.load_chapter_sync();
            }

            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                if self.sidebar_open {
                    self.reading_panel_open = false;
                    self.menu_open = false;
                }
            }
            Message::OpenSearchSidebar => {
                self.sidebar_open = true;
                self.sidebar_tab = SidebarTab::Search;
                self.reading_panel_open = false;
                self.menu_open = false;
            }
            Message::SelectSidebarTab(tab) => {
                self.sidebar_tab = tab;
            }
            Message::SelectTocEntry(idx) => {
                let hrefs: Vec<String> = self.toc_entries.iter().map(|e| e.href.clone()).collect();
                if let Some(href) = hrefs.get(idx) {
                    if let Some(ref mut doc) = self.document {
                        if doc.goto_toc_href(href) {
                            self.spine_progress = (doc.spine_index + 1, doc.spine_len());
                            self.sidebar_open = false;
                            self.showing_image = false;
                            self.reset_reader_scroll();
                            self.load_chapter_sync();
                        }
                    }
                }
            }

            Message::SearchQueryChanged(q) => {
                self.search_query = q;
            }
            Message::ExecuteSearch => {
                let query = self.search_query.trim().to_string();
                if query.is_empty() || self.document.is_none() {
                    self.search_results.clear();
                    return Task::none();
                }
                if let Some(ref doc) = self.document {
                    match doc.search(&query) {
                        Ok(results) => self.search_results = results,
                        Err(_) => self.search_results.clear(),
                    }
                }
            }
            Message::SelectSearchResult(idx) => {
                if let Some(result) = self.search_results.get(idx).cloned() {
                    if let Some(ref mut doc) = self.document {
                        doc.goto_chapter_id(&result.chapter_id);
                        self.spine_progress = (doc.spine_index + 1, doc.spine_len());
                        self.sidebar_open = false;
                        self.search_results.clear();
                        self.search_query.clear();
                        self.showing_image = false;
                        self.reset_reader_scroll();
                        self.load_chapter_sync();
                    }
                }
            }

            Message::ToggleReadingPanel => {
                self.reading_panel_open = !self.reading_panel_open;
                if self.reading_panel_open {
                    self.sidebar_open = false;
                    self.menu_open = false;
                }
            }
            Message::SetProfile(key) => {
                self.settings.current_profile = key;
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::FontIncrease => {
                let current = (self.settings.font_size_pt / DEFAULT_READER_FONT_SIZE_PT * 10.0)
                    .round()
                    * 10.0;
                let percent = (current + 10.0).min(200.0);
                self.settings.font_size_pt = DEFAULT_READER_FONT_SIZE_PT * percent / 100.0;
                self.update_style_map_sizes();
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::FontDecrease => {
                let current = (self.settings.font_size_pt / DEFAULT_READER_FONT_SIZE_PT * 10.0)
                    .round()
                    * 10.0;
                let percent = (current - 10.0).max(60.0);
                self.settings.font_size_pt = DEFAULT_READER_FONT_SIZE_PT * percent / 100.0;
                self.update_style_map_sizes();
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::FontReset => {
                self.settings.font_size_pt = DEFAULT_READER_FONT_SIZE_PT;
                self.update_style_map_sizes();
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::SelectFont(name) => {
                self.settings.font_family = name;
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::LineHeightIncrease => {
                self.settings.line_height = (self.settings.line_height + 0.1).min(2.5);
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::LineHeightDecrease => {
                self.settings.line_height = (self.settings.line_height - 0.1).max(1.0);
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::MarginIncrease => {
                self.settings.margin_em = (self.settings.margin_em + 0.5).min(6.0);
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }
            Message::MarginDecrease => {
                self.settings.margin_em = (self.settings.margin_em - 0.5).max(0.5);
                self.invalidate_reader_layout();
                let _ = self.settings.save();
            }

            Message::ToggleMenu => {
                self.menu_open = !self.menu_open;
                if self.menu_open {
                    self.reading_panel_open = false;
                    self.sidebar_open = false;
                }
            }
            Message::ToggleTts => {
                if self.tts_engine.is_speaking() {
                    let _ = self.tts_engine.stop();
                } else {
                    let text = self.plain_text();
                    if !text.is_empty() {
                        let _ = self.tts_engine.speak(&text);
                    }
                }
            }

            Message::ImageClicked(src) => {
                let img_path = PathBuf::from(&src);
                if img_path.exists() {
                    let _ = self.visore_state.load_image(&img_path);
                    self.showing_image = true;
                } else {
                    self.status = Some(format!("Imagen no encontrada: {}", src));
                }
            }
            Message::CloseImage => {
                self.showing_image = false;
            }

            Message::SetReaderScroll(scroll_y) => {
                let metrics = self.reader_metrics.borrow();
                let max_scroll = (metrics.total_h - metrics.viewport_h).max(0.0);
                self.reader_scroll_y = scroll_y.clamp(0.0, max_scroll);
            }

            Message::PageDown => {
                let metrics = self.reader_metrics.borrow();
                let page = metrics.viewport_h * 0.9;
                let max_scroll = (metrics.total_h - metrics.viewport_h).max(0.0);
                self.reader_scroll_y = (self.reader_scroll_y + page).min(max_scroll);
            }

            Message::PageUp => {
                let metrics = self.reader_metrics.borrow();
                let page = metrics.viewport_h * 0.9;
                self.reader_scroll_y = (self.reader_scroll_y - page).max(0.0);
            }

            Message::KeyPressed(key_event) => {
                if let keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat,
                    ..
                } = key_event
                {
                    if modifiers.command() && !repeat {
                        match key.as_ref() {
                            Key::Character(value) if value.eq_ignore_ascii_case("o") => {
                                return self.update(Message::OpenFile);
                            }
                            Key::Character(value) => {
                                if let Some(index) = recent_shortcut_index(value) {
                                    if let Some(path) = self.settings.recent_book(index) {
                                        return self.update(Message::FileSelected(Some(path)));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if self.showing_image || self.document.is_none() {
                        return Task::none();
                    }
                    match key {
                        Key::Named(keyboard::key::Named::ArrowRight) => {
                            if self.reader_at_end() {
                                return self.update(Message::NextChapter);
                            } else {
                                return self.update(Message::PageDown);
                            }
                        }
                        Key::Named(keyboard::key::Named::ArrowLeft) => {
                            if self.reader_at_start() {
                                return self.update(Message::PrevChapter);
                            } else {
                                return self.update(Message::PageUp);
                            }
                        }
                        _ => {}
                    }
                }
            }

            Message::Visore(vmsg) => match vmsg {
                visore::viewer::ViewerMessage::Cancel => self.showing_image = false,
                visore::viewer::ViewerMessage::SetTheme(t) => self.visore_state.theme = t,
                visore::viewer::ViewerMessage::SetAspectRatio(r) => {
                    self.visore_state.aspect_ratio = r
                }
                visore::viewer::ViewerMessage::SetOrientation(o) => {
                    self.visore_state.orientation = o
                }
                visore::viewer::ViewerMessage::RotateCW => self.visore_state.rotate_cw(),
                visore::viewer::ViewerMessage::RotateCCW => self.visore_state.rotate_ccw(),
                visore::viewer::ViewerMessage::FlipH => self.visore_state.toggle_flip_h(),
                visore::viewer::ViewerMessage::FlipV => self.visore_state.toggle_flip_v(),
                _ => {}
            },
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if self.showing_image {
            return visore::viewer::viewer_view(&self.visore_state, false).map(Message::Visore);
        }

        let profile = self.settings.current_profile();
        let bg_color =
            parse_hex_color(&profile.bg_color).unwrap_or(Color::from_rgb8(250, 248, 245));
        let text_color =
            parse_hex_color(&profile.text_color).unwrap_or(Color::from_rgb8(26, 26, 26));

        let has_book = self.document.is_some();

        let body: Element<'_, Message> = if self.sidebar_open {
            let panel = crate::ui::sidebar::view_sidebar(
                &self.sidebar_tab,
                &self.toc_entries,
                &self.search_query,
                &self.search_results,
            );
            let reader = crate::ui::reader_view::view_reader(
                has_book,
                &self.current_blocks,
                &self.style_map,
                &self.settings,
                &self.font_name_map,
                &self.css_rules,
                bg_color,
                self.scroll_id.clone(),
                self.reader_scroll_y,
                self.reader_metrics.clone(),
                self.image_metadata_cache.clone(),
                self.reader_layout_cache.clone(),
            );
            cosmic::widget::Row::with_children([panel, reader])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else if self.reading_panel_open {
            let panel =
                crate::ui::reading_panel::view_reading_panel(&self.settings, &self.epub_fonts);
            let reader = crate::ui::reader_view::view_reader(
                has_book,
                &self.current_blocks,
                &self.style_map,
                &self.settings,
                &self.font_name_map,
                &self.css_rules,
                bg_color,
                self.scroll_id.clone(),
                self.reader_scroll_y,
                self.reader_metrics.clone(),
                self.image_metadata_cache.clone(),
                self.reader_layout_cache.clone(),
            );
            cosmic::widget::Row::with_children([panel, reader])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            crate::ui::reader_view::view_reader(
                has_book,
                &self.current_blocks,
                &self.style_map,
                &self.settings,
                &self.font_name_map,
                &self.css_rules,
                bg_color,
                self.scroll_id.clone(),
                self.reader_scroll_y,
                self.reader_metrics.clone(),
                self.image_metadata_cache.clone(),
                self.reader_layout_cache.clone(),
            )
        };
        let footer: Element<'_, Message> = if has_book {
            crate::ui::footer::view_footer(
                self.spine_progress,
                self.reader_chapter_percent(),
                self.reader_book_percent(),
                bg_color,
                text_color,
            )
        } else {
            cosmic::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Shrink)
                .into()
        };

        cosmic::widget::Column::with_children([body, footer])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _id| match event {
            Event::Keyboard(key_event) => {
                if status == EventStatus::Ignored {
                    Some(Message::KeyPressed(key_event))
                } else {
                    None
                }
            }
            _ => None,
        })
    }
}

impl App {
    fn plain_text(&self) -> String {
        self.current_blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Heading { spans, .. } | ContentBlock::Paragraph { spans, .. } => {
                    Some(
                        spans
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                }
                ContentBlock::Image { alt, .. } => Some(if alt.is_empty() {
                    "[Imagen]".into()
                } else {
                    format!("[{}]", alt)
                }),
                ContentBlock::Separator => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn load_chapter_sync(&mut self) {
        if let Some(ref mut doc) = self.document {
            match doc.current_chapter_blocks() {
                Ok(blocks) => {
                    self.current_blocks = blocks;
                    self.chapter_title = extract_heading(&self.current_blocks);
                }
                Err(e) => {
                    self.current_blocks = vec![ContentBlock::Paragraph {
                        spans: vec![StyledSpan {
                            text: format!("Error: {}", e),
                            bold: false,
                            italic: false,
                            underline: false,
                            strikethrough: false,
                            color: Some(Color::from_rgb8(200, 50, 50)),
                            size: None,
                            link: None,
                            classes: vec![],
                        }],
                        classes: vec![],
                    }];
                    self.chapter_title.clear();
                }
            }
            let title = if self.chapter_title.is_empty() {
                "Folio".to_string()
            } else {
                self.chapter_title.clone()
            };
            self.set_header_title(title);
        }
    }

    fn update_style_map_sizes(&mut self) {
        let base = self.settings.font_size_pt as f32;
        self.style_map.p.font_size = base;
        self.style_map.h1.font_size = base * 1.86;
        self.style_map.h2.font_size = base * 1.57;
        self.style_map.h3.font_size = base * 1.29;
        self.style_map.h4.font_size = base * 1.14;
        self.style_map.h5.font_size = base;
        self.style_map.h6.font_size = base * 0.93;
    }

    fn load_epub_css(
        &self,
        doc: &DocumentModel,
    ) -> (Vec<CssRule>, Vec<(FontFaceRule, std::path::PathBuf)>) {
        let mut all_rules = Vec::new();
        let mut all_faces = Vec::new();
        for (_id, item) in &doc.core.manifest {
            if item.media_type.contains("css") {
                if let Ok(path) = doc.core.get_resource_path(&item.href) {
                    let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                    if let Ok(css_text) = std::fs::read_to_string(&path) {
                        let (rules, faces) = css::parse_css(&css_text);
                        all_rules.extend(rules);
                        for face in faces {
                            all_faces.push((face, parent.clone()));
                        }
                    }
                }
            }
        }
        (all_rules, all_faces)
    }

    // ── Footer ──

    // ── Sidebar (TOC / Search) ──

    // ── Reading panel (Aa) ──

    // ── Reader area ──

    fn reset_reader_scroll(&mut self) {
        self.reader_scroll_y = 0.0;
        *self.reader_metrics.borrow_mut() = ReaderMetrics::default();
        self.invalidate_reader_layout();
    }

    fn invalidate_reader_layout(&self) {
        self.reader_layout_cache.borrow_mut().clear();
    }

    fn reader_at_start(&self) -> bool {
        let metrics = self.reader_metrics.borrow();
        if metrics.total_h == 0.0 {
            return false;
        }
        self.reader_scroll_y <= 1.0
    }

    fn reader_at_end(&self) -> bool {
        let metrics = self.reader_metrics.borrow();
        if metrics.total_h == 0.0 {
            return false;
        }
        if metrics.total_h <= metrics.viewport_h {
            return true;
        }
        self.reader_scroll_y + metrics.viewport_h >= metrics.total_h - 1.0
    }

    fn reader_chapter_percent(&self) -> f32 {
        let metrics = self.reader_metrics.borrow();
        if metrics.total_h == 0.0 {
            return 0.0;
        }
        if metrics.total_h <= metrics.viewport_h {
            return 100.0;
        }
        let max_scroll = metrics.total_h - metrics.viewport_h;
        (self.reader_scroll_y / max_scroll * 100.0).clamp(0.0, 100.0)
    }

    fn reader_book_percent(&self) -> f32 {
        let (cur, total) = self.spine_progress;
        if total == 0 {
            return 0.0;
        }
        let chapter_pct = self.reader_chapter_percent() / 100.0;
        ((cur.saturating_sub(1) as f32 + chapter_pct) / total as f32 * 100.0).clamp(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::recent_shortcut_index;

    #[test]
    fn maps_ctrl_number_shortcuts_to_recent_book_indexes() {
        assert_eq!(recent_shortcut_index("1"), Some(0));
        assert_eq!(recent_shortcut_index("9"), Some(8));
        assert_eq!(recent_shortcut_index("0"), None);
        assert_eq!(recent_shortcut_index("10"), None);
        assert_eq!(recent_shortcut_index("o"), None);
    }
}
