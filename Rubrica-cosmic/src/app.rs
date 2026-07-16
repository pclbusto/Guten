use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::app::{Core, Task};
use cosmic::iced::event::{self, Event, Status as EventStatus};
use cosmic::iced::keyboard;
use cosmic::iced::{Alignment, Color, Length, Subscription};
use cosmic::widget::about::About;
use cosmic::widget::segmented_button;
use cosmic::widget::{button, icon, pane_grid, text, text_editor, tooltip};
use cosmic::{Application, ApplicationExt, Element};

use crate::document::ProjectModel;
use crate::settings::AppSettings;
use crate::ui;
use crate::ui::export_dialog::view_export_dialog;
use crate::ui::rename_dialog::view_rename_dialog;
use crate::ui::toc_dialog::view_toc_dialog;

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
    CloseTab(String),
    EditorAction(text_editor::Action),
    SaveCurrent,
    SaveTab(String),
    Saved(Result<String, String>),

    OpenExport,
    SelectExportFormat(ExportFormat),
    ConfirmExport,
    CancelExport,
    ExportPathSelected(ExportFormat, Option<PathBuf>),

    ToggleSidebar,
    SelectSidebarTab(crate::app::SidebarTab),
    SidebarTabSelected(segmented_button::Entity),
    TogglePreviewPanel,
    PreviewPanelResized(pane_grid::ResizeEvent),
    SidebarResourceSelected(String),
    SidebarDragStart(String),
    SidebarDragOver(String),
    SidebarDragLeave(String),
    SidebarDrop,
    SetReaderScroll(f32),
    ReaderWheel(f32),
    ImageClicked(String),

    SearchQueryChanged(String),
    ExecuteSearch,
    SearchResultsLoaded(Result<Vec<SearchResult>, String>),
    SelectSearchResult(usize),

    CreateChapter,
    CreateStyle,
    ImportImage,
    ImportFont,
    ImageFileSelected(Option<PathBuf>),
    FontFileSelected(Option<PathBuf>),

    SetCover(String),
    StartRename(String),
    RenameInputChanged(String),
    ConfirmRename,
    CancelRename,

    OpenTocDialog,
    CloseTocDialog,
    TocDialogDocInclude(usize, bool),
    TocDialogHeadingInclude(usize, usize, bool),
    TocDialogSelectAll(bool),
    GenerateToc,

    KeyPressed(keyboard::Event),
    OpenUrl(String),
    ToggleAbout,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorPane {
    Editor,
    Preview,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    #[default]
    Epub,
    Text,
}

pub(crate) struct OpenTab {
    pub resource_id: String,
    pub media_type: String,
    pub content: text_editor::Content,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TocDialog {
    pub data: Vec<gutencore::DocToc>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenameDialog {
    pub item_id: String,
    pub current_href: String,
    pub new_href: String,
}

pub(crate) struct App {
    core: Core,
    project: Option<ProjectModel>,
    settings: AppSettings,

    sidebar_open: bool,
    sidebar_tab: SidebarTab,
    sidebar_tab_model: segmented_button::SingleSelectModel,
    sidebar_width: f32,
    resource_nav_model: segmented_button::SingleSelectModel,
    preview_panel_open: bool,
    preview_panel_ratio: f32,
    preview_panes: Option<pane_grid::State<EditorPane>>,
    sidebar_drag_id: Option<String>,
    sidebar_drop_id: Option<String>,
    resource_groups: Vec<crate::document::ResourceGroup>,

    tabs: Vec<OpenTab>,
    active_tab_id: Option<String>,

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

    show_about: bool,
    about: About,
    status: Option<String>,

    toc_dialog: Option<TocDialog>,
    rename_dialog: Option<RenameDialog>,
    export_dialog: Option<ExportFormat>,
}

fn perform_async<T: Send + 'static>(
    future: impl std::future::Future<Output = T> + Send + 'static,
    f: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message> {
    cosmic::task::future(async move { f(future.await) })
}

fn build_sidebar_tab_model(active: SidebarTab) -> segmented_button::SingleSelectModel {
    let mut model = segmented_button::Model::default();
    let resources_id = model.insert().text("Recursos").id();
    let search_id = model.insert().text("Buscar").id();
    model.activate(match active {
        SidebarTab::Resources => resources_id,
        SidebarTab::Search => search_id,
    });
    model
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
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        items.push(
            tooltip(
                button::icon(icon::from_name("open-menu-symbolic"))
                    .on_press(Message::ToggleSidebar)
                    .padding(8),
                text::body("Mostrar/ocultar panel"),
                tooltip::Position::Bottom,
            )
            .into(),
        );
        items.push(
            tooltip(
                button::icon(icon::from_name("document-open-symbolic"))
                    .on_press(Message::OpenProject)
                    .padding(8),
                text::body("Abrir proyecto"),
                tooltip::Position::Bottom,
            )
            .into(),
        );
        items.push(
            tooltip(
                button::icon(icon::from_name("document-new-symbolic"))
                    .on_press(Message::NewProject)
                    .padding(8),
                text::body("Nuevo proyecto"),
                tooltip::Position::Bottom,
            )
            .into(),
        );

        if self.project.is_some() {
            items.push(
                tooltip(
                    button::icon(icon::from_name("document-save-symbolic"))
                        .on_press(Message::SaveCurrent)
                        .padding(8),
                    text::body("Guardar"),
                    tooltip::Position::Bottom,
                )
                .into(),
            );
            items.push(
                tooltip(
                    button::icon(icon::from_name("document-send-symbolic"))
                        .on_press(Message::OpenExport)
                        .padding(8),
                    text::body("Exportar"),
                    tooltip::Position::Bottom,
                )
                .into(),
            );
            items.push(
                tooltip(
                    button::icon(icon::from_name("window-close-symbolic"))
                        .on_press(Message::CloseProject)
                        .padding(8),
                    text::body("Cerrar proyecto"),
                    tooltip::Position::Bottom,
                )
                .into(),
            );
        }

        items
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            tooltip(
                button::icon(icon::from_name("help-about-symbolic"))
                    .on_press(Message::ToggleAbout)
                    .padding(8),
                text::body("Acerca de"),
                tooltip::Position::Bottom,
            )
            .into(),
        ]
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Self::Message>> {
        self.show_about.then(|| {
            context_drawer::about(
                &self.about,
                |url| Message::OpenUrl(url.to_owned()),
                Message::ToggleAbout,
            )
        })
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let settings = AppSettings::load();
        let sidebar_width = settings.sidebar_width;

        let about = About::default()
            .name("Rúbrica")
            .icon(icon::from_name(Self::APP_ID))
            .version(env!("CARGO_PKG_VERSION"))
            .author("Guten")
            .comments("Editor de libros EPUB para COSMIC, parte del proyecto Guten.")
            .copyright("© 2026 Proyecto Guten")
            .license("MIT")
            .license_url("https://github.com/GutenAir/Rubrica-cosmic/blob/main/LICENSE")
            .links([
                ("Repositorio", "https://github.com/GutenAir/Rubrica-cosmic"),
                (
                    "Documentación",
                    "https://github.com/GutenAir/Rubrica-cosmic/blob/main/README.md",
                ),
                ("Proyecto Guten", "https://github.com/GutenAir"),
                (
                    "Reportar un problema",
                    "https://github.com/GutenAir/Rubrica-cosmic/issues",
                ),
            ]);

        let mut app = Self {
            core,
            project: None,
            settings,
            sidebar_open: true,
            sidebar_tab: SidebarTab::Resources,
            sidebar_tab_model: build_sidebar_tab_model(SidebarTab::Resources),
            sidebar_width,
            resource_nav_model: segmented_button::Model::default(),
            preview_panel_open: false,
            preview_panel_ratio: 0.5,
            preview_panes: None,
            sidebar_drag_id: None,
            sidebar_drop_id: None,
            resource_groups: Vec::new(),
            tabs: Vec::new(),
            active_tab_id: None,
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
            show_about: false,
            about,
            status: None,

            toc_dialog: None,
            rename_dialog: None,
            export_dialog: None,
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
                        self.settings
                            .remember_recent_project(&project.original_path);
                        let _ = self.settings.save();
                        let title = project.title();
                        self.project = Some(project);
                        self.rebuild_resource_nav_model();
                        self.tabs.clear();
                        self.active_tab_id = None;
                        self.search_results.clear();
                        self.search_query.clear();
                        self.status = None;
                        self.preview_panel_open = false;
                        self.preview_panes = None;
                        self.sidebar_tab = SidebarTab::Resources;
                        self.sidebar_tab_model = build_sidebar_tab_model(SidebarTab::Resources);
                        self.show_about = false;
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
                self.perform_close_project();
                self.status = Some("Proyecto cerrado".into());
                return cosmic::task::future(async {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    Message::StatusCleared
                });
            }
            Message::OpenExport => {
                self.export_dialog = Some(ExportFormat::Epub);
            }
            Message::SelectExportFormat(format) => {
                self.export_dialog = Some(format);
            }
            Message::CancelExport => {
                self.export_dialog = None;
            }
            Message::ConfirmExport => {
                let Some(format) = self.export_dialog.take() else {
                    return Task::none();
                };
                let (label, extension) = match format {
                    ExportFormat::Epub => ("EPUB", "epub"),
                    ExportFormat::Text => ("Texto plano", "txt"),
                };
                return perform_async(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter(label, &[extension])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |path| Message::ExportPathSelected(format, path),
                );
            }
            Message::ExportPathSelected(format, Some(mut path)) => {
                if let Err(error) = self.save_dirty_tabs() {
                    self.status = Some(format!("Error exportando: {}", error));
                    return Task::none();
                }
                if let Some(project) = self.project.as_mut() {
                    let extension = match format {
                        ExportFormat::Epub => "epub",
                        ExportFormat::Text => "txt",
                    };
                    path.set_extension(extension);

                    let result = match format {
                        ExportFormat::Epub => project.core.export_epub(&path).map(|_| path),
                        ExportFormat::Text => {
                            let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
                            let filename = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned());
                            project.core.export_to_text_file(parent, filename, None)
                        }
                    };

                    match result {
                        Ok(_) => {
                            self.status = Some(match format {
                                ExportFormat::Epub => "EPUB exportado".into(),
                                ExportFormat::Text => "Texto plano exportado".into(),
                            });
                            return cosmic::task::future(async {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                Message::StatusCleared
                            });
                        }
                        Err(e) => {
                            self.status = Some(format!("Error exportando: {:#}", e));
                        }
                    }
                }
            }
            Message::ExportPathSelected(_, None) => {}

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
                self.activate_resource_in_nav(&id);
                if self.find_tab_index(&id).is_some() {
                    self.set_active_tab(&id);
                } else if let Some(ref project) = self.project {
                    let media_type = project
                        .core
                        .manifest
                        .get(&id)
                        .map(|item| item.media_type.clone())
                        .unwrap_or_default();
                    if ProjectModel::is_text_editable(&media_type) {
                        match project.load_resource_text(&id) {
                            Ok(text) => {
                                self.tabs.push(OpenTab {
                                    resource_id: id.clone(),
                                    media_type,
                                    content: text_editor::Content::with_text(&text),
                                    dirty: false,
                                });
                                self.set_active_tab(&id);
                            }
                            Err(e) => {
                                self.status = Some(format!("Error cargando recurso: {}", e));
                            }
                        }
                    }
                }
            }
            Message::CloseTab(id) => {
                self.close_tab(&id);
            }

            Message::EditorAction(action) => {
                if let Some(tab) = self.active_tab_mut() {
                    tab.content.perform(action);
                    tab.dirty = true;
                    self.invalidate_reader_layout();
                }
            }

            Message::SaveCurrent => {
                let id = self.active_tab_id.clone().unwrap_or_default();
                return Task::done(cosmic::Action::App(Message::SaveTab(id)));
            }
            Message::SaveTab(id) => {
                let result = self
                    .save_dirty_tabs()
                    .and_then(|_| {
                        self.project
                            .as_mut()
                            .ok_or_else(|| "No hay un proyecto abierto".to_string())?
                            .save()
                            .map_err(|e| format!("{:#}", e))
                    })
                    .map(|_| id);
                return self.update(Message::Saved(result));
            }
            Message::Saved(Ok(id)) => {
                if let Some(tab) = self.find_tab_mut(&id) {
                    tab.dirty = false;
                }
                let reloaded_css = self
                    .find_tab(&id)
                    .is_some_and(|tab| tab.media_type == "text/css");
                if reloaded_css {
                    self.load_project_assets();
                    self.invalidate_reader_layout();
                }
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
                self.sidebar_tab_model = build_sidebar_tab_model(tab);
            }
            Message::SidebarTabSelected(entity) => {
                let tab = if self.sidebar_tab_model.active() == entity {
                    self.sidebar_tab
                } else {
                    match entity {
                        _ if self.sidebar_tab_model.text(entity) == Some("Recursos") => {
                            SidebarTab::Resources
                        }
                        _ if self.sidebar_tab_model.text(entity) == Some("Buscar") => {
                            SidebarTab::Search
                        }
                        _ => self.sidebar_tab,
                    }
                };
                return self.update(Message::SelectSidebarTab(tab));
            }
            Message::TogglePreviewPanel => {
                self.preview_panel_open = !self.preview_panel_open;
                if self.preview_panel_open {
                    self.load_project_assets();
                    self.invalidate_reader_layout();
                    let config = pane_grid::Configuration::Split {
                        axis: pane_grid::Axis::Vertical,
                        ratio: self.preview_panel_ratio,
                        a: Box::new(pane_grid::Configuration::Pane(EditorPane::Editor)),
                        b: Box::new(pane_grid::Configuration::Pane(EditorPane::Preview)),
                    };
                    self.preview_panes = Some(pane_grid::State::with_configuration(config));
                } else {
                    self.preview_panes = None;
                }
            }
            Message::PreviewPanelResized(event) => {
                self.preview_panel_ratio = event.ratio;
                if let Some(ref mut panes) = self.preview_panes {
                    panes.resize(event.split, event.ratio);
                }
            }
            Message::SidebarResourceSelected(id) => {
                return self.update(Message::SelectResource(id));
            }
            Message::SidebarDragStart(id) => {
                eprintln!("[SidebarDragStart] id={}", id);
                self.sidebar_drag_id = Some(id);
                self.sidebar_drop_id = None;
            }
            Message::SidebarDragOver(id) => {
                if self.sidebar_drag_id.is_some() {
                    eprintln!("[SidebarDragOver] drag_id={:?} drop_id={}", self.sidebar_drag_id, id);
                    self.sidebar_drop_id = Some(id);
                }
            }
            Message::SidebarDragLeave(id) => {
                // No limpiamos drop_id aquí: durante un drag pueden llegar
                // eventos enter/exit muy rápidos y perder el target justo antes
                // del release. El drop se encarga de limpiar el estado.
                eprintln!("[SidebarDragLeave] drop_id={}", id);
            }
            Message::SidebarDrop => {
                eprintln!(
                    "[SidebarDrop] drag_id={:?} drop_id={:?}",
                    self.sidebar_drag_id, self.sidebar_drop_id
                );
                if let Some(project) = self.project.as_mut() {
                    if let (Some(drag_id), Some(drop_id)) =
                        (self.sidebar_drag_id.clone(), self.sidebar_drop_id.clone())
                    {
                        if drag_id != drop_id {
                            let drag_pos = project.core.spine.iter().position(|id| id == &drag_id);
                            let drop_pos = project.core.spine.iter().position(|id| id == &drop_id);
                            eprintln!(
                                "[SidebarDrop] drag_pos={:?} drop_pos={:?} spine={:?}",
                                drag_pos, drop_pos, project.core.spine
                            );
                            if let (Some(drag_pos), Some(drop_pos)) = (drag_pos, drop_pos) {
                                let new_index = if drag_pos < drop_pos {
                                    drop_pos - 1
                                } else {
                                    drop_pos
                                };
                                eprintln!("[SidebarDrop] new_index={}", new_index);
                                let move_result = project.core.spine_move(&drag_id, new_index);
                                eprintln!("[SidebarDrop] spine_move result={:?}", move_result);
                                let result = move_result.and_then(|_| {
                                    project
                                        .rebuild_navigation_from_spine()
                                        .map_err(|e| gutencore::GutenError::Manifest(format!("{}", e)))
                                });
                                eprintln!("[SidebarDrop] rebuild_navigation result={:?}", result);
                                match result {
                                    Ok(_) => self.status = Some("Orden actualizado".into()),
                                    Err(e) => {
                                        self.status = Some(format!("Error reordenando: {}", e))
                                    }
                                }
                                self.rebuild_resource_nav_model();
                            }
                        }
                    }
                }
                self.sidebar_drag_id = None;
                self.sidebar_drop_id = None;
            }
            Message::SetReaderScroll(scroll_y) => {
                let metrics = self.reader_metrics.borrow();
                let max_scroll = (metrics.total_h - metrics.viewport_h).max(0.0);
                self.reader_scroll_y = scroll_y.clamp(0.0, max_scroll);
            }
            Message::ReaderWheel(delta) => {
                let metrics = self.reader_metrics.borrow();
                let max_scroll = (metrics.total_h - metrics.viewport_h).max(0.0);
                self.reader_scroll_y = (self.reader_scroll_y + delta).clamp(0.0, max_scroll);
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
                            return self
                                .update(Message::SearchResultsLoaded(Err(format!("{}", e))));
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

            Message::CreateChapter => {
                if let Some(project) = self.project.as_mut() {
                    let id = project.unique_id("chapter");
                    let title = format!("Capítulo {}", id.trim_start_matches("chapter"));
                    match project.create_chapter(&id, &title) {
                        Ok(()) => {
                            self.status = Some(format!("Capítulo {} creado", id));
                            return self.refresh_resources_and_select(&id);
                        }
                        Err(e) => {
                            self.status = Some(format!("Error creando capítulo: {}", e));
                        }
                    }
                }
            }
            Message::CreateStyle => {
                if let Some(project) = self.project.as_mut() {
                    let id = project.unique_id("style");
                    match project.create_style(&id) {
                        Ok(()) => {
                            self.status = Some(format!("Estilo {} creado", id));
                            return self.refresh_resources_and_select(&id);
                        }
                        Err(e) => {
                            self.status = Some(format!("Error creando estilo: {}", e));
                        }
                    }
                }
            }
            Message::ImportImage => {
                return perform_async(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Imágenes", &["png", "jpg", "jpeg", "gif", "svg", "webp"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ImageFileSelected,
                );
            }
            Message::ImageFileSelected(Some(path)) => {
                if let Some(project) = self.project.as_mut() {
                    match project.import_image(&path) {
                        Ok(id) => {
                            self.status = Some(format!("Imagen {} importada", id));
                            return self.refresh_resources_and_select(&id);
                        }
                        Err(e) => {
                            self.status = Some(format!("Error importando imagen: {}", e));
                        }
                    }
                }
            }
            Message::ImageFileSelected(None) => {}
            Message::ImportFont => {
                return perform_async(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Fuentes", &["ttf", "otf", "woff", "woff2"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::FontFileSelected,
                );
            }
            Message::FontFileSelected(Some(path)) => {
                if let Some(project) = self.project.as_mut() {
                    match project.import_font(&path) {
                        Ok(id) => {
                            self.status = Some(format!("Fuente {} importada", id));
                            return self.refresh_resources_and_select(&id);
                        }
                        Err(e) => {
                            self.status = Some(format!("Error importando fuente: {}", e));
                        }
                    }
                }
            }
            Message::FontFileSelected(None) => {}

            Message::SetCover(id) => {
                if let Some(ref mut project) = self.project {
                    match project.set_cover_image(&id) {
                        Ok(()) => {
                            self.status = Some("Portada configurada".into());
                            self.rebuild_resource_nav_model();
                        }
                        Err(e) => {
                            self.status = Some(format!("Error configurando portada: {:#}", e));
                        }
                    }
                }
            }
            Message::StartRename(id) => {
                if let Some(ref project) = self.project
                    && let Some(item) = project.core.manifest.get(&id)
                {
                    self.rename_dialog = Some(RenameDialog {
                        item_id: id,
                        current_href: item.href.clone(),
                        new_href: item.href.clone(),
                    });
                }
            }
            Message::RenameInputChanged(new_val) => {
                if let Some(ref mut dialog) = self.rename_dialog {
                    dialog.new_href = new_val;
                }
            }
            Message::ConfirmRename => {
                if let Some(ref mut project) = self.project {
                    if let Some(dialog) = self.rename_dialog.take()
                        && dialog.new_href != dialog.current_href
                    {
                        match project.rename_resource(&dialog.item_id, &dialog.new_href) {
                            Ok(()) => {
                                self.status = Some("Recurso renombrado".into());
                                self.rebuild_resource_nav_model();
                                self.load_project_assets();
                            }
                            Err(e) => {
                                self.status =
                                    Some(format!("Error renombrando: {}", e));
                            }
                        }
                    }
                } else {
                    self.rename_dialog = None;
                }
            }
            Message::CancelRename => {
                self.rename_dialog = None;
            }

            Message::OpenTocDialog => {
                if let Some(project) = self.project.as_ref() {
                    match project.get_toc_data() {
                        Ok((data, errors)) => {
                            if data.is_empty() {
                                self.status = Some(
                                    "No hay documentos XHTML para escanear".into(),
                                );
                            } else {
                                self.toc_dialog = Some(TocDialog { data });
                                if !errors.is_empty() {
                                    self.status = Some(format!(
                                        "TOC: {} no escaneado ({})",
                                        errors.len(),
                                        errors.first().cloned().unwrap_or_default()
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            self.status = Some(format!("Error escaneando TOC: {}", e));
                        }
                    }
                }
            }
            Message::CloseTocDialog => {
                self.toc_dialog = None;
            }
            Message::TocDialogDocInclude(doc_idx, include) => {
                if let Some(dialog) = self.toc_dialog.as_mut() {
                    if let Some(doc) = dialog.data.get_mut(doc_idx) {
                        doc.include = include;
                        if !include {
                            for h in &mut doc.items {
                                h.include = false;
                            }
                        }
                    }
                }
            }
            Message::TocDialogHeadingInclude(doc_idx, heading_idx, include) => {
                if let Some(dialog) = self.toc_dialog.as_mut() {
                    if let Some(doc) = dialog.data.get_mut(doc_idx) {
                        if let Some(heading) = doc.items.get_mut(heading_idx) {
                            heading.include = include;
                        }
                        if include {
                            doc.include = true;
                        }
                    }
                }
            }
            Message::TocDialogSelectAll(include) => {
                if let Some(dialog) = self.toc_dialog.as_mut() {
                    for doc in &mut dialog.data {
                        doc.include = include;
                        for h in &mut doc.items {
                            h.include = include;
                        }
                    }
                }
            }
            Message::GenerateToc => {
                if let Some(dialog) = self.toc_dialog.take() {
                    if let Some(project) = self.project.as_mut() {
                        match project.build_navigation(&dialog.data, true) {
                            Ok(()) => {
                                self.status = Some("Tabla de contenidos generada".into());
                                self.rebuild_resource_nav_model();
                            }
                            Err(e) => {
                                self.status = Some(format!("Error generando TOC: {}", e));
                                self.toc_dialog = Some(dialog);
                            }
                        }
                    }
                }
            }

            Message::KeyPressed(key_event) => {
                if let keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat,
                    ..
                } = key_event
                    && modifiers.command()
                    && !repeat
                {
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

            Message::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    self.status = Some(format!("No se pudo abrir el enlace: {}", e));
                }
            }
            Message::ToggleAbout => {
                self.show_about = !self.show_about;
                self.set_show_context(self.show_about);
            }

            Message::StatusCleared => {
                self.status = None;
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let has_project = self.project.is_some();

        let body: Element<'_, Message> = if has_project {
            let content_area: Element<'_, Message> = if self.preview_panel_open {
                if let Some(panes) = self.preview_panes.as_ref() {
                    pane_grid::PaneGrid::new(
                        panes,
                        |_pane, pane_kind, _maximized| match pane_kind {
                            EditorPane::Editor => {
                                pane_grid::Content::new(ui::editor_view::view_editor(
                                    has_project,
                                    self.active_tab(),
                                    self.active_tab_id.as_deref(),
                                    self.preview_panel_open,
                                    &self.tabs,
                                    &self.settings.recent_projects,
                                ))
                            }
                            EditorPane::Preview => pane_grid::Content::new(self.preview_content()),
                        },
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .on_resize(10, Message::PreviewPanelResized)
                    .into()
                } else {
                    ui::editor_view::view_editor(
                        has_project,
                        self.active_tab(),
                        self.active_tab_id.as_deref(),
                        self.preview_panel_open,
                        &self.tabs,
                        &self.settings.recent_projects,
                    )
                }
            } else {
                ui::editor_view::view_editor(
                    has_project,
                    self.active_tab(),
                    self.active_tab_id.as_deref(),
                    self.preview_panel_open,
                    &self.tabs,
                    &self.settings.recent_projects,
                )
            };

            if self.sidebar_open {
                let sidebar = ui::resource_sidebar::view_sidebar(
                    &self.sidebar_tab_model,
                    &self.resource_groups,
                    self.active_tab_id.as_deref(),
                    self.sidebar_drag_id.as_deref(),
                    self.sidebar_drop_id.as_deref(),
                    &self.search_query,
                    &self.search_results,
                    self.sidebar_width,
                );
                cosmic::widget::Row::with_children([sidebar, content_area])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                content_area
            }
        } else {
            ui::editor_view::view_editor(
                has_project,
                self.active_tab(),
                self.active_tab_id.as_deref(),
                self.preview_panel_open,
                &self.tabs,
                &self.settings.recent_projects,
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

        if let Some(dialog) = self.toc_dialog.as_ref() {
            return view_toc_dialog(&dialog.data);
        }

        if let Some(format) = self.export_dialog {
            return view_export_dialog(format);
        }

        if let Some(dialog) = self.rename_dialog.as_ref() {
            return view_rename_dialog(dialog);
        }

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

    fn find_tab_index(&self, resource_id: &str) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.resource_id == resource_id)
    }

    fn find_tab(&self, resource_id: &str) -> Option<&OpenTab> {
        self.tabs.iter().find(|tab| tab.resource_id == resource_id)
    }

    fn find_tab_mut(&mut self, resource_id: &str) -> Option<&mut OpenTab> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.resource_id == resource_id)
    }

    fn active_tab(&self) -> Option<&OpenTab> {
        self.active_tab_id.as_ref().and_then(|id| self.find_tab(id))
    }

    fn active_tab_mut(&mut self) -> Option<&mut OpenTab> {
        let id = self.active_tab_id.clone()?;
        self.find_tab_mut(&id)
    }

    fn set_active_tab(&mut self, resource_id: &str) {
        self.active_tab_id = Some(resource_id.to_string());
        self.activate_resource_in_nav(resource_id);
        self.invalidate_reader_layout();
    }

    fn close_tab(&mut self, resource_id: &str) {
        let Some(idx) = self.find_tab_index(resource_id) else {
            return;
        };
        if self.tabs[idx].dirty
            && let Some(ref mut project) = self.project
        {
            let content = self.tabs[idx].content.text();
            let _ = project.save_resource_text(resource_id, &content);
        }
        self.tabs.remove(idx);
        if self.active_tab_id.as_deref() == Some(resource_id) {
            self.active_tab_id = self
                .tabs
                .get(idx.saturating_sub(1))
                .map(|tab| tab.resource_id.clone());
        }
    }

    fn save_dirty_tabs(&mut self) -> Result<(), String> {
        let Some(ref mut project) = self.project else {
            return Ok(());
        };
        for tab in &mut self.tabs {
            if tab.dirty {
                let content = tab.content.text();
                project
                    .save_resource_text(&tab.resource_id, &content)
                    .map_err(|e| format!("{:#}", e))?;
                tab.dirty = false;
            }
        }
        Ok(())
    }

    fn refresh_resources_and_select(&mut self, resource_id: &str) -> Task<Message> {
        self.rebuild_resource_nav_model();
        self.update(Message::SelectResource(resource_id.to_string()))
    }

    fn rebuild_resource_nav_model(&mut self) {
        self.resource_nav_model.clear();
        self.resource_groups.clear();
        let Some(project) = self.project.as_ref() else {
            return;
        };
        self.resource_groups = project.grouped_resources();
        let mut first_group = true;
        for group in &self.resource_groups {
            let header_id = self
                .resource_nav_model
                .insert()
                .text(group.label)
                .divider_above(!first_group)
                .id();
            self.resource_nav_model.enable(header_id, false);
            for item in &group.items {
                let item_id = self
                    .resource_nav_model
                    .insert()
                    .text(item.id.clone())
                    .data(item.id.clone())
                    .id();
                if self.active_tab_id.as_ref() == Some(&item.id) {
                    self.resource_nav_model.activate(item_id);
                }
            }
            first_group = false;
        }
    }

    fn activate_resource_in_nav(&mut self, resource_id: &str) {
        let entities: Vec<_> = self.resource_nav_model.iter().collect();
        for entity in entities {
            if let Some(id) = self.resource_nav_model.data::<String>(entity)
                && id == resource_id
            {
                self.resource_nav_model.activate(entity);
                return;
            }
        }
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

    fn preview_content(&self) -> Element<'_, Message> {
        let Some(tab) = self.active_tab() else {
            return cosmic::widget::container(text::body(
                "Seleccioná un recurso del panel lateral para ver la vista previa",
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into();
        };

        ui::editor_view::view_preview(
            tab.content.text(),
            self.project.as_ref(),
            Some(&tab.resource_id),
            Some(&tab.media_type),
            &self.style_map,
            &self.css_rules,
            &self.font_name_map,
            self.reader_scroll_y,
            self.reader_metrics.clone(),
            self.image_metadata_cache.clone(),
            self.reader_layout_cache.clone(),
            self.preview_bg_color,
            self.preview_text_color,
        )
    }

    fn perform_close_project(&mut self) {
        let _ = self.save_dirty_tabs();
        self.project = None;
        self.resource_nav_model.clear();
        self.resource_groups.clear();
        self.tabs.clear();
        self.active_tab_id = None;
        self.search_results.clear();
        self.search_query.clear();
        self.status = None;
        self.sidebar_tab = SidebarTab::Resources;
        self.sidebar_tab_model = build_sidebar_tab_model(SidebarTab::Resources);
        self.preview_panel_open = false;
        self.preview_panes = None;
        self.show_about = false;
        self.toc_dialog = None;
        self.export_dialog = None;
        self.set_show_context(false);
        self.reset_reader_state();
        self.css_rules.clear();
        self.font_faces.clear();
        self.epub_fonts.clear();
        self.font_name_map = folio::fonts::FontNameMap::default();
        let _ = self.settings.save();
        self.set_header_title("Rúbrica".into());
    }
}
