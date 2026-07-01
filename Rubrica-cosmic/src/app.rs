use cosmic::app::{Core, Task};
use cosmic::iced::event::{self, Event, Status as EventStatus};
use cosmic::iced::keyboard;
use cosmic::iced::{Alignment, Color, Length, Subscription};
use cosmic::widget::{button, text, text_editor};
use cosmic::{Application, ApplicationExt, Element};

use crate::document::ProjectModel;
use crate::settings::AppSettings;
use crate::ui;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    OpenProject,
    ProjectSelected(Option<PathBuf>),
    CloseProject,

    NewProject,
    NewProjectCreated(Result<PathBuf, String>),
    OpenRecent(usize),

    SelectResource(String),
    ResourceLoaded(Result<(String, String), String>),
    EditorAction(text_editor::Action),
    SaveCurrent,
    Saved(Result<(), String>),

    ToggleSidebar,
    SelectSidebarTab(crate::app::SidebarTab),
    SelectEditorTab(EditorTab),
    SetReaderScroll(f32),
    ImageClicked(String),

    SearchQueryChanged(String),
    ExecuteSearch,
    SearchResultsLoaded(Result<Vec<SearchResult>, String>),
    SelectSearchResult(usize),

    KeyPressed(keyboard::Event),
    StatusCleared,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub chapter_id: String,
    pub snippet: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTab {
    #[default]
    Resources,
    Search,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorTab {
    #[default]
    Editor,
    Preview,
}

pub(crate) struct App {
    core: Core,
    project: Option<ProjectModel>,
    settings: AppSettings,

    sidebar_open: bool,
    sidebar_tab: SidebarTab,
    sidebar_width: f32,
    active_tab: EditorTab,

    selected_resource_id: Option<String>,
    selected_resource_media_type: Option<String>,
    editor_content: text_editor::Content,
    editor_dirty: bool,

    search_query: String,
    search_results: Vec<SearchResult>,

    reader_scroll_y: f32,
    reader_metrics: Rc<RefCell<folio::reader::text_canvas::ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<folio::image_resources::ImageMetadataCache>>,
    reader_layout_cache: Rc<RefCell<folio::reader::text_canvas::ReaderLayoutCache>>,
    style_map: folio::content::StyleMap,
    css_rules: Vec<folio::css::CssRule>,
    font_faces: Vec<(folio::css::FontFaceRule, PathBuf)>,
    epub_fonts: Vec<folio::fonts::EpubFont>,
    font_name_map: folio::fonts::FontNameMap,
    preview_bg_color: Color,
    preview_text_color: Color,

    status: Option<String>,
}

fn perform_async<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
    f: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    cosmic::task::future(async move { f(future.await) })
}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = Option<PathBuf>;
    type Message = Message;
    const APP_ID: &'static str = "com.gutenair.RubricaCosmic";

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
        let mut items: Vec<Element<'_, Message>> = Vec::new();
        items.push(
            button::standard("Abrir (Ctrl+O)")
                .on_press(Message::OpenProject)
                .into(),
        );
        items.push(button::standard("Nuevo").on_press(Message::NewProject).into());
        if self.project.is_some() {
            items.push(
                button::standard("Guardar (Ctrl+S)")
                    .on_press(Message::SaveCurrent)
                    .into(),
            );
            items.push(
                button::standard("Cerrar")
                    .on_press(Message::CloseProject)
                    .into(),
            );
        }
        items
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let settings = AppSettings::load();
        let sidebar_width = settings.sidebar_width;

        let mut app = Self {
            core,
            project: None,
            settings,
            sidebar_open: true,
            sidebar_tab: SidebarTab::Resources,
            sidebar_width,
            active_tab: EditorTab::Editor,
            selected_resource_id: None,
            selected_resource_media_type: None,
            editor_content: text_editor::Content::new(),
            editor_dirty: false,
            search_query: String::new(),
            search_results: Vec::new(),
            reader_scroll_y: 0.0,
            reader_metrics: Rc::new(RefCell::new(
                folio::reader::text_canvas::ReaderMetrics::default(),
            )),
            image_metadata_cache: Rc::new(RefCell::new(
                folio::image_resources::ImageMetadataCache::default(),
            )),
            reader_layout_cache: Rc::new(RefCell::new(
                folio::reader::text_canvas::ReaderLayoutCache::default(),
            )),
            style_map: folio::content::StyleMap::default(),
            css_rules: Vec::new(),
            font_faces: Vec::new(),
            epub_fonts: Vec::new(),
            font_name_map: folio::fonts::FontNameMap::default(),
            preview_bg_color: Color::from_rgb8(250, 248, 245),
            preview_text_color: Color::from_rgb8(26, 26, 26),
            status: None,
        };

        app.set_header_title("Rúbrica".into());

        let task = if let Some(path) = flags {
            Task::done(cosmic::Action::App(Message::ProjectSelected(Some(path))))
        } else {
            Task::none()
        };
        (app, task)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::OpenProject => {
                return perform_async(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("EPUB", &["epub"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ProjectSelected,
                );
            }

            Message::ProjectSelected(Some(path)) => {
                self.status = Some("Abriendo proyecto...".into());
                match ProjectModel::open(&path) {
                    Ok(project) => {
                        self.settings.remember_recent_project(&project.original_path);
                        let _ = self.settings.save();
                        let title = project.title();
                        self.project = Some(project);
                        self.selected_resource_id = None;
                        self.selected_resource_media_type = None;
                        self.editor_content = text_editor::Content::new();
                        self.editor_dirty = false;
                        self.search_results.clear();
                        self.search_query.clear();
                        self.status = None;
                        self.active_tab = EditorTab::Editor;
                        self.reset_reader_state();
                        self.load_project_assets();
                        self.set_header_title(title);
                    }
                    Err(e) => {
                        self.status = Some(format!("Error: {}", e));
                    }
                }
            }
            Message::ProjectSelected(None) => {}

            Message::CloseProject => {
                if self.editor_dirty {
                    if let Some(ref mut project) = self.project {
                        if let Some(ref id) = self.selected_resource_id {
                            let content = self.editor_content.text();
                            let _ = project.save_resource_text(id, &content);
                        }
                    }
                }
                self.project = None;
                self.selected_resource_id = None;
                self.selected_resource_media_type = None;
                self.editor_content = text_editor::Content::new();
                self.editor_dirty = false;
                self.search_results.clear();
                self.search_query.clear();
                self.status = None;
                self.reset_reader_state();
                self.css_rules.clear();
                self.font_faces.clear();
                self.epub_fonts.clear();
                self.font_name_map = folio::fonts::FontNameMap::default();
                self.set_header_title("Rúbrica".into());
            }

            Message::NewProject => {
                return perform_async(
                    async {
                        let handle = rfd::AsyncFileDialog::new().pick_folder().await?;
                        let folder = handle.path().to_path_buf();
                        if std::fs::read_dir(&folder).ok()?.next().is_some() {
                            return Some(Err("El directorio debe estar vacío".into()));
                        }
                        match ProjectModel::create_new(&folder, "Nuevo libro", "es") {
                            Ok(_) => Some(Ok(folder)),
                            Err(e) => Some(Err(format!("{}", e))),
                        }
                    },
                    |result| {
                        Message::NewProjectCreated(match result {
                            Some(Ok(path)) => Ok(path),
                            Some(Err(e)) => Err(e),
                            None => Err("Cancelado".into()),
                        })
                    },
                );
            }
            Message::NewProjectCreated(Ok(path)) => {
                return self.update(Message::ProjectSelected(Some(path)));
            }
            Message::NewProjectCreated(Err(e)) => {
                self.status = Some(format!("Error: {}", e));
            }
            Message::OpenRecent(idx) => {
                if let Some(path) = self.settings.recent_projects.get(idx).cloned() {
                    return self.update(Message::ProjectSelected(Some(path)));
                }
            }

            Message::SelectResource(id) => {
                if let Some(ref project) = self.project {
                    let media_type = project
                        .core
                        .manifest
                        .get(&id)
                        .map(|item| item.media_type.clone());
                    self.selected_resource_media_type = media_type.clone();
                    self.active_tab = EditorTab::Editor;

                    if ProjectModel::is_text_editable(media_type.as_deref().unwrap_or("")) {
                        self.selected_resource_id = Some(id.clone());
                        let result = project
                            .load_resource_text(&id)
                            .map(|text| (id, text))
                            .map_err(|e| format!("{}", e));
                        return self.update(Message::ResourceLoaded(result));
                    } else {
                        self.selected_resource_id = Some(id);
                        self.editor_content = text_editor::Content::with_text(
                            "Este recurso no es editable como texto.",
                        );
                        self.editor_dirty = false;
                    }
                }
            }
            Message::ResourceLoaded(Ok((id, text))) => {
                if self.selected_resource_id.as_ref() == Some(&id) {
                    self.editor_content = text_editor::Content::with_text(&text);
                    self.editor_dirty = false;
                }
            }
            Message::ResourceLoaded(Err(e)) => {
                self.status = Some(format!("Error cargando recurso: {}", e));
            }

            Message::EditorAction(action) => {
                self.editor_content.perform(action);
                self.editor_dirty = true;
                self.invalidate_reader_layout();
            }

            Message::SaveCurrent => {
                if let Some(ref mut project) = self.project {
                    if let Some(ref id) = self.selected_resource_id {
                        let content = self.editor_content.text();
                        let result = project.save_resource_text(id, &content).map_err(|e| format!("{}", e));
                        return self.update(Message::Saved(result));
                    }
                }
            }
            Message::Saved(Ok(())) => {
                self.editor_dirty = false;
                self.status = Some("Guardado".into());
                return cosmic::task::future(async {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    Message::StatusCleared
                });
            }
            Message::Saved(Err(e)) => {
                self.status = Some(format!("Error guardando: {}", e));
            }

            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
            }
            Message::SelectSidebarTab(tab) => {
                self.sidebar_tab = tab;
            }
            Message::SelectEditorTab(tab) => {
                self.active_tab = tab;
                if tab == EditorTab::Preview {
                    self.invalidate_reader_layout();
                }
            }
            Message::SetReaderScroll(scroll_y) => {
                let metrics = self.reader_metrics.borrow();
                let max_scroll = (metrics.total_h - metrics.viewport_h).max(0.0);
                self.reader_scroll_y = scroll_y.clamp(0.0, max_scroll);
            }
            Message::ImageClicked(src) => {
                self.status = Some(format!("Imagen: {}", src));
            }

            Message::SearchQueryChanged(q) => {
                self.search_query = q;
            }
            Message::ExecuteSearch => {
                let query = self.search_query.trim().to_string();
                if query.is_empty() {
                    self.search_results.clear();
                    return Task::none();
                }
                if let Some(ref project) = self.project {
                    let results: Vec<SearchResult> = match project.core.search(&query) {
                        Ok(items) => items
                            .into_iter()
                            .map(|r| SearchResult {
                                chapter_id: r.chapter_id,
                                snippet: r.snippet,
                            })
                            .collect(),
                        Err(e) => {
                            return self.update(Message::SearchResultsLoaded(Err(format!("{}", e))));
                        }
                    };
                    return self.update(Message::SearchResultsLoaded(Ok(results)));
                }
            }
            Message::SearchResultsLoaded(Ok(results)) => {
                self.search_results = results;
            }
            Message::SearchResultsLoaded(Err(e)) => {
                self.status = Some(format!("Error en búsqueda: {}", e));
            }
            Message::SelectSearchResult(idx) => {
                if let Some(result) = self.search_results.get(idx).cloned() {
                    self.sidebar_tab = SidebarTab::Resources;
                    return self.update(Message::SelectResource(result.chapter_id));
                }
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
                            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("o") => {
                                return self.update(Message::OpenProject);
                            }
                            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("s") => {
                                return self.update(Message::SaveCurrent);
                            }
                            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("n") => {
                                return self.update(Message::NewProject);
                            }
                            _ => {}
                        }
                    }
                }
            }

            Message::StatusCleared => {
                self.status = None;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let has_project = self.project.is_some();

        let body: Element<'_, Message> = if self.sidebar_open && has_project {
            let sidebar = ui::resource_sidebar::view_sidebar(
                &self.sidebar_tab,
                self.project.as_ref().unwrap(),
                &self.selected_resource_id,
                &self.search_query,
                &self.search_results,
                self.sidebar_width,
            );
            let editor = ui::editor_view::view_editor(
                has_project,
                self.project.as_ref(),
                self.selected_resource_id.as_deref(),
                self.selected_resource_media_type.as_deref(),
                &self.editor_content,
                self.editor_dirty,
                self.active_tab,
                &self.settings.recent_projects,
                self.reader_scroll_y,
                self.reader_metrics.clone(),
                self.image_metadata_cache.clone(),
                self.reader_layout_cache.clone(),
                &self.style_map,
                &self.css_rules,
                &self.font_name_map,
                self.preview_bg_color,
                self.preview_text_color,
            );
            cosmic::widget::Row::with_children([sidebar, editor])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            ui::editor_view::view_editor(
                has_project,
                self.project.as_ref(),
                self.selected_resource_id.as_deref(),
                self.selected_resource_media_type.as_deref(),
                &self.editor_content,
                self.editor_dirty,
                self.active_tab,
                &self.settings.recent_projects,
                self.reader_scroll_y,
                self.reader_metrics.clone(),
                self.image_metadata_cache.clone(),
                self.reader_layout_cache.clone(),
                &self.style_map,
                &self.css_rules,
                &self.font_name_map,
                self.preview_bg_color,
                self.preview_text_color,
            )
        };

        let footer: Element<'_, Message> = if let Some(ref status) = self.status {
            cosmic::widget::container(text::body(status.clone()).size(12.0))
                .width(Length::Fill)
                .padding([4, 16])
                .align_x(Alignment::Center)
                .into()
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
    fn reset_reader_state(&mut self) {
        self.reader_scroll_y = 0.0;
        *self.reader_metrics.borrow_mut() = folio::reader::text_canvas::ReaderMetrics::default();
        self.reader_layout_cache.borrow_mut().clear();
        self.image_metadata_cache.borrow_mut().clear();
        self.style_map = folio::content::StyleMap::default();
    }

    fn invalidate_reader_layout(&self) {
        self.reader_layout_cache.borrow_mut().clear();
    }

    fn load_project_assets(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };

        self.epub_fonts = folio::fonts::extract_epub_fonts(&project.core.manifest, |id| {
            project.core.get_resource_path(id)
        });
        let (rules, faces) = self.load_epub_css(project);
        self.font_name_map = folio::fonts::build_font_name_map(&faces, &self.epub_fonts);
        self.css_rules = rules;
        self.font_faces = faces;
        self.update_style_map_sizes();
    }

    fn load_epub_css(
        &self,
        project: &ProjectModel,
    ) -> (
        Vec<folio::css::CssRule>,
        Vec<(folio::css::FontFaceRule, PathBuf)>,
    ) {
        let mut all_rules = Vec::new();
        let mut all_faces = Vec::new();
        for item in project.core.manifest.values() {
            if item.media_type.contains("css") {
                let Ok(path) = project.core.get_resource_path(&item.href) else {
                    continue;
                };
                let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                let Ok(css_text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let (rules, faces) = folio::css::parse_css(&css_text);
                all_rules.extend(rules);
                for face in faces {
                    all_faces.push((face, parent.clone()));
                }
            }
        }
        (all_rules, all_faces)
    }

    fn update_style_map_sizes(&mut self) {
        let base = self.settings.editor_font_size;
        self.style_map.p.font_size = base;
        self.style_map.h1.font_size = base * 1.86;
        self.style_map.h2.font_size = base * 1.57;
        self.style_map.h3.font_size = base * 1.29;
        self.style_map.h4.font_size = base * 1.14;
        self.style_map.h5.font_size = base;
        self.style_map.h6.font_size = base * 0.93;
    }

}
