use cosmic::app::{Core, Task};
use cosmic::iced::widget::{center, column, container, row, scrollable, stack, text};
use cosmic::iced::{
    Alignment, Background, Border, Color, ContentFit, Length, Shadow, Subscription, Vector,
    keyboard,
};
use cosmic::widget::{button, divider, mouse_area, text_input};
use cosmic::{ApplicationExt, Element, executor};
use scriptorium::analytics::{Analytics, LibraryMetrics};
use scriptorium::library_db::{BookDetail, BookListItem, LibraryDb};
use scriptorium::pipeline::{ImportStatus, Pipeline};
use scriptorium::sync::SyncSubsystem;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const GRID_COLUMNS: usize = 6;
const DEFAULT_COVERS_PER_PAGE: usize = 24;
const COVERS_PER_PAGE_OPTIONS: [usize; 5] = [12, 24, 48, 72, 96];
const SHOWCASE_CARDS: usize = 12;
const CARD_HEIGHT: f32 = 326.0;
const CARD_SLOT_HEIGHT: f32 = CARD_HEIGHT + 2.0;
const COVER_LOAD_CONCURRENCY: usize = 4;
const COVER_THUMBNAIL_WIDTH: u32 = 360;
const COVER_THUMBNAIL_HEIGHT: u32 = 540;
const COVER_CACHE_VERSION: &str = "v2-thumb-360x540";
const OPDS_SERVER_PORT: u16 = 8080;
static COVER_CACHE_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Library,
    List,
    Stats,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Page {
    Main,
    Detail(BookDetail),
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    total_books: i64,
    reading_time_secs: i64,
    authors: usize,
    series: usize,
    tags: usize,
}

#[derive(Clone)]
pub struct DbHandle(LibraryDb);

impl fmt::Debug for DbHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DbHandle")
    }
}

#[derive(Debug, Clone)]
pub struct UiBook {
    id: i64,
    title: String,
    author_name: String,
    series_name: Option<String>,
    current_path: String,
    is_normalized: bool,
    cover_image: Option<cosmic::iced::widget::image::Handle>,
    cover_loading: bool,
}

impl From<BookListItem> for UiBook {
    fn from(book: BookListItem) -> Self {
        Self {
            id: book.id,
            title: book.title,
            author_name: book.author_name,
            series_name: book.series_name,
            current_path: book.current_path,
            is_normalized: book.is_normalized,
            cover_image: None,
            cover_loading: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedBooks {
    books: Vec<UiBook>,
    raw_books: Vec<BookListItem>,
    total_count: usize,
    page: usize,
}

#[derive(Debug, Clone)]
pub struct SystemApp {
    id: String,
    name: String,
}

pub struct App {
    core: Core,
    db: Option<LibraryDb>,
    page: Page,
    section: Section,
    books: Vec<UiBook>,
    total_books_count: usize,
    library_page: usize,
    cover_cache: HashMap<String, Option<cosmic::iced::widget::image::Handle>>,
    stats: Stats,
    search: String,
    search_active: bool,
    search_id: cosmic::widget::Id,
    title_edit: String,
    author_edit: String,
    show_edit_dialog: bool,
    show_delete_dialog: bool,
    selected_card_design: String,
    hovered_card: Option<(i64, &'static str)>,
    loading: bool,
    status: Option<String>,
    pending_status: Option<String>,
    covers_per_page: usize,
    opds_server_enabled: bool,
    epub_apps: Vec<SystemApp>,
    reader_app_id: Option<String>,
    editor_app_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    DatabaseOpened(Result<DbHandle, String>),
    BooksLoaded(Result<LoadedBooks, String>),
    StatsLoaded(Result<Stats, String>),
    SectionSelected(Section),
    SearchChanged(String),
    SearchOpened,
    SearchStarted(String),
    SearchClosed,
    SearchSubmitted,
    ImportPressed,
    FilesSelected(Result<Vec<PathBuf>, String>),
    ImportFinished(Result<usize, String>),
    OpenDetail(i64),
    DetailLoaded(Result<Option<BookDetail>, String>),
    TitleChanged(String),
    AuthorChanged(String),
    SaveMetadata,
    OpenEditDialog,
    CloseEditDialog,
    MetadataSaved(Result<(), String>),
    OpenDeleteDialog,
    CloseDeleteDialog,
    DeleteBook { delete_file: bool },
    BookDeleted(Result<Option<String>, String>),
    ReadBook,
    BookOpened(Result<(), String>),
    EditBook,
    BookEdited(Result<(), String>),
    QuickAction(&'static str),
    CardDesignSelected(String),
    PageSizeSelected(usize),
    LibraryPageSelected(usize),
    CoverLoaded(Result<(i64, String, Option<cosmic::iced::widget::image::Handle>), String>),
    CardHoverChanged(Option<(i64, &'static str)>),
    EnableOpdsServer,
    OpdsServerEnabled(Result<String, String>),
    ReaderAppSelected(Option<String>),
    EditorAppSelected(Option<String>),
}

impl cosmic::Application for App {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.scriptorium.Scriptorium";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut app = App {
            core,
            db: None,
            page: Page::Main,
            section: Section::Library,
            books: Vec::new(),
            total_books_count: 0,
            library_page: 0,
            cover_cache: HashMap::new(),
            stats: Stats::default(),
            search: String::new(),
            search_active: false,
            search_id: cosmic::widget::Id::unique(),
            title_edit: String::new(),
            author_edit: String::new(),
            show_edit_dialog: false,
            show_delete_dialog: false,
            selected_card_design: load_card_design_preference(),
            hovered_card: None,
            loading: true,
            status: None,
            pending_status: None,
            covers_per_page: load_page_size_preference(),
            opds_server_enabled: false,
            epub_apps: discover_epub_apps(),
            reader_app_id: load_app_preference(AppRole::Reader),
            editor_app_id: load_app_preference(AppRole::Editor),
        };

        app.set_header_title("Scriptorium".into());
        let title_task = if let Some(id) = app.core.main_window_id() {
            app.set_window_title("Scriptorium".into(), id)
        } else {
            Task::none()
        };
        let open_task = perform(open_database(), Message::DatabaseOpened);

        (app, cosmic::task::batch([title_task, open_task]))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        update_app(self, message)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let library_selected = self.section == Section::Library;
        let list_selected = self.section == Section::List;
        let stats_selected = self.section == Section::Stats;
        let settings_selected = self.section == Section::Settings;

        vec![
            header_icon_button(
                "view-grid-symbolic",
                "Biblioteca",
                library_selected,
                Message::SectionSelected(Section::Library),
            ),
            header_icon_button(
                "view-list-symbolic",
                "Lista",
                list_selected,
                Message::SectionSelected(Section::List),
            ),
            header_icon_button(
                "utilities-system-monitor-symbolic",
                "Estadísticas",
                stats_selected,
                Message::SectionSelected(Section::Stats),
            ),
            header_icon_button(
                "preferences-system-symbolic",
                "Configuración",
                settings_selected,
                Message::SectionSelected(Section::Settings),
            ),
        ]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut actions = Vec::new();
        if self.search_active && matches!(&self.page, Page::Main) {
            actions.push(
                text_input::search_input("Buscar por título o autor…", &self.search)
                    .on_input(Message::SearchChanged)
                    .on_submit(|_| Message::SearchSubmitted)
                    .id(self.search_id.clone())
                    .width(320)
                    .into(),
            );
            actions.push(header_icon_button(
                "window-close-symbolic",
                "Cerrar búsqueda",
                false,
                Message::SearchClosed,
            ));
        } else {
            actions.push(header_icon_button(
                "system-search-symbolic",
                "Buscar",
                false,
                Message::SearchOpened,
            ));
        }
        actions.push(header_icon_button(
            "list-add-symbolic",
            "Importar EPUB",
            false,
            Message::ImportPressed,
        ));
        actions
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        if let Page::Detail(book) = &self.page {
            if self.show_delete_dialog {
                return Some(
                    cosmic::widget::dialog()
                        .title("Eliminar libro")
                        .body(format!(
                            "¿Qué querés hacer con “{}”? Quitar de la biblioteca conserva el archivo EPUB en disco.",
                            book.title
                        ))
                        .primary_action(
                            button::destructive("Quitar de la biblioteca")
                                .on_press(Message::DeleteBook { delete_file: false }),
                        )
                        .secondary_action(
                            button::destructive("Eliminar archivo también")
                                .on_press(Message::DeleteBook { delete_file: true }),
                        )
                        .tertiary_action(
                            button::standard("Cancelar").on_press(Message::CloseDeleteDialog),
                        )
                        .into(),
                );
            }

            if self.show_edit_dialog {
                return Some(
                    cosmic::widget::dialog()
                        .title("Editar metadatos")
                        .body(
                            "Los cambios disponibles actualmente se limitan al título y al autor.",
                        )
                        .control(
                            text_input::text_input("Título", &self.title_edit)
                                .on_input(Message::TitleChanged),
                        )
                        .control(
                            text_input::text_input("Autor", &self.author_edit)
                                .on_input(Message::AuthorChanged),
                        )
                        .primary_action(
                            button::suggested("Guardar cambios").on_press(Message::SaveMetadata),
                        )
                        .secondary_action(
                            button::standard("Cancelar").on_press(Message::CloseEditDialog),
                        )
                        .into(),
                );
            }
        }
        None
    }

    fn on_search(&mut self) -> Task<Self::Message> {
        if !matches!(&self.page, Page::Main)
            || !matches!(self.section, Section::Library | Section::List)
        {
            return Task::none();
        }
        self.search_active = true;
        cosmic::widget::text_input::focus(self.search_id.clone())
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        if self.show_delete_dialog {
            self.show_delete_dialog = false;
        } else if self.show_edit_dialog {
            self.show_edit_dialog = false;
        } else if self.search_active {
            self.search_active = false;
            self.search.clear();
            if let Some(db) = self.db.clone() {
                self.loading = true;
                return perform(
                    load_books(db, None, 0, self.covers_per_page),
                    Message::BooksLoaded,
                );
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.search_active
            || !matches!(&self.page, Page::Main)
            || !matches!(self.section, Section::Library | Section::List)
        {
            return Subscription::none();
        }

        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                text: Some(text),
                modifiers,
                ..
            } if !modifiers.control()
                && !modifiers.alt()
                && !modifiers.logo()
                && text
                    .chars()
                    .any(|character| !character.is_control() && !character.is_whitespace()) =>
            {
                Some(Message::SearchStarted(text.to_string()))
            }
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Self::Message> {
        view_app(self)
    }
}

fn header_icon_button(
    icon_name: &'static str,
    tooltip: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    button::icon(cosmic::widget::icon::from_name(icon_name).size(14))
        .icon_size(14)
        .line_height(16)
        .padding(2)
        .tooltip(tooltip)
        .selected(selected)
        .on_press(message)
        .into()
}

fn perform<T>(
    future: impl Future<Output = T> + Send + 'static,
    map: impl FnOnce(T) -> Message + Send + 'static,
) -> Task<Message>
where
    T: Send + 'static,
{
    cosmic::task::future(async move { map(future.await) })
}

fn update_app(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::DatabaseOpened(result) => match result {
            Ok(DbHandle(db)) => {
                app.db = Some(db.clone());
                app.loading = true;
                perform(
                    load_books(db, None, 0, app.covers_per_page),
                    Message::BooksLoaded,
                )
            }
            Err(error) => {
                app.loading = false;
                app.status = Some(error);
                Task::none()
            }
        },
        Message::BooksLoaded(result) => {
            app.loading = false;
            match result {
                Ok(loaded) => {
                    app.books = loaded.books;
                    app.total_books_count = loaded.total_count;
                    app.library_page = loaded.page;
                    app.hovered_card = None;
                    app.status = app.pending_status.take();
                    let semaphore = Arc::new(tokio::sync::Semaphore::new(COVER_LOAD_CONCURRENCY));
                    let mut cover_tasks = Vec::new();
                    for book in loaded.raw_books {
                        let key = cover_cache_key(&book);
                        if let Some(cover) = app.cover_cache.get(&key) {
                            if let Some(ui_book) =
                                app.books.iter_mut().find(|ui_book| ui_book.id == book.id)
                            {
                                ui_book.cover_image = cover.clone();
                                ui_book.cover_loading = false;
                            }
                            continue;
                        }
                        cover_tasks.push(perform(
                            load_cover(book, key, semaphore.clone()),
                            Message::CoverLoaded,
                        ));
                    }
                    return Task::batch(cover_tasks);
                }
                Err(error) => app.status = Some(error),
            }
            Task::none()
        }
        Message::CoverLoaded(result) => {
            if let Ok((book_id, key, cover)) = result {
                if let Some(book) = app.books.iter_mut().find(|book| book.id == book_id) {
                    book.cover_image = cover.clone();
                    book.cover_loading = false;
                }
                app.cover_cache.insert(key, cover);
            }
            Task::none()
        }
        Message::StatsLoaded(result) => {
            app.loading = false;
            match result {
                Ok(stats) => {
                    app.stats = stats;
                    app.status = None;
                }
                Err(error) => app.status = Some(error),
            }
            Task::none()
        }
        Message::SectionSelected(section) => {
            app.section = section;
            app.page = Page::Main;
            app.hovered_card = None;
            app.show_edit_dialog = false;
            app.show_delete_dialog = false;
            if !matches!(section, Section::Library | Section::List) {
                app.search_active = false;
                app.search.clear();
            }
            app.loading = true;
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            if section == Section::Stats {
                perform(load_stats(db), Message::StatsLoaded)
            } else {
                let query = normalized_query(&app.search);
                perform(
                    load_books(db, query, 0, app.covers_per_page),
                    Message::BooksLoaded,
                )
            }
        }
        Message::SearchChanged(query) => {
            app.search = query;
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            perform(
                load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                Message::BooksLoaded,
            )
        }
        Message::SearchOpened => {
            let needs_library = !matches!(&app.page, Page::Main)
                || !matches!(app.section, Section::Library | Section::List);
            if needs_library {
                app.page = Page::Main;
                app.section = Section::Library;
                app.show_edit_dialog = false;
            }
            app.search_active = true;
            let focus = cosmic::widget::text_input::focus(app.search_id.clone());
            if !needs_library {
                return focus;
            }
            let Some(db) = app.db.clone() else {
                return focus;
            };
            app.loading = true;
            cosmic::task::batch([
                focus,
                perform(
                    load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                    Message::BooksLoaded,
                ),
            ])
        }
        Message::SearchStarted(text) => {
            app.search_active = true;
            app.search.push_str(&text);
            let focus = cosmic::widget::text_input::focus(app.search_id.clone());
            let Some(db) = app.db.clone() else {
                return focus;
            };
            app.loading = true;
            cosmic::task::batch([
                focus,
                perform(
                    load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                    Message::BooksLoaded,
                ),
            ])
        }
        Message::SearchClosed => {
            app.search_active = false;
            app.search.clear();
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            perform(
                load_books(db, None, 0, app.covers_per_page),
                Message::BooksLoaded,
            )
        }
        Message::SearchSubmitted => {
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            perform(
                load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                Message::BooksLoaded,
            )
        }
        Message::ImportPressed => perform(select_epubs(), Message::FilesSelected),
        Message::FilesSelected(Ok(paths)) if !paths.is_empty() => {
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            app.status = Some("Importando libros…".into());
            perform(import_books(db, paths), Message::ImportFinished)
        }
        Message::FilesSelected(Ok(_)) => {
            app.status = Some("No se seleccionaron archivos EPUB.".into());
            Task::none()
        }
        Message::FilesSelected(Err(error)) => {
            app.status = Some(error);
            Task::none()
        }
        Message::ImportFinished(result) => match result {
            Ok(count) => {
                app.status = Some(format!("{count} libro(s) importado(s)."));
                let Some(db) = app.db.clone() else {
                    return Task::none();
                };
                perform(
                    load_books(db, None, 0, app.covers_per_page),
                    Message::BooksLoaded,
                )
            }
            Err(error) => {
                app.loading = false;
                app.status = Some(error);
                Task::none()
            }
        },
        Message::OpenDetail(id) => {
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.hovered_card = None;
            app.loading = true;
            perform(load_detail(db, id), Message::DetailLoaded)
        }
        Message::DetailLoaded(result) => {
            app.loading = false;
            match result {
                Ok(Some(book)) => {
                    app.title_edit = book.title.clone();
                    app.author_edit = book.author_name.clone();
                    app.page = Page::Detail(book);
                    app.show_delete_dialog = false;
                    app.status = None;
                }
                Ok(None) => app.status = Some("El libro ya no existe.".into()),
                Err(error) => app.status = Some(error),
            }
            Task::none()
        }
        Message::TitleChanged(value) => {
            app.title_edit = value;
            Task::none()
        }
        Message::AuthorChanged(value) => {
            app.author_edit = value;
            Task::none()
        }
        Message::SaveMetadata => {
            let (Some(db), Page::Detail(book)) = (app.db.clone(), &app.page) else {
                return Task::none();
            };
            app.loading = true;
            perform(
                save_metadata(db, book.id, app.title_edit.clone(), app.author_edit.clone()),
                Message::MetadataSaved,
            )
        }
        Message::MetadataSaved(result) => {
            app.loading = false;
            match result {
                Ok(()) => {
                    app.show_edit_dialog = false;
                    if let Page::Detail(book) = &mut app.page {
                        book.title = app.title_edit.clone();
                        book.author_name = app.author_edit.clone();
                    }
                    app.status = Some("Metadatos guardados.".into());
                }
                Err(error) => app.status = Some(error),
            }
            Task::none()
        }
        Message::OpenEditDialog => {
            app.show_edit_dialog = true;
            app.show_delete_dialog = false;
            Task::none()
        }
        Message::CloseEditDialog => {
            app.show_edit_dialog = false;
            Task::none()
        }
        Message::OpenDeleteDialog => {
            app.show_delete_dialog = true;
            app.show_edit_dialog = false;
            Task::none()
        }
        Message::CloseDeleteDialog => {
            app.show_delete_dialog = false;
            Task::none()
        }
        Message::DeleteBook { delete_file } => {
            let (Some(db), Page::Detail(book)) = (app.db.clone(), &app.page) else {
                return Task::none();
            };
            app.loading = true;
            app.show_delete_dialog = false;
            perform(delete_book(db, book.id, delete_file), Message::BookDeleted)
        }
        Message::BookDeleted(result) => {
            app.loading = false;
            match result {
                Ok(deleted_file) => {
                    app.page = Page::Main;
                    app.section = Section::Library;
                    app.show_edit_dialog = false;
                    app.show_delete_dialog = false;
                    app.status = if let Some(path) = deleted_file {
                        Some(format!("Libro eliminado y archivo borrado: {path}"))
                    } else {
                        Some("Libro quitado de la biblioteca.".into())
                    };
                    app.pending_status = app.status.clone();
                    app.hovered_card = None;
                    let Some(db) = app.db.clone() else {
                        return Task::none();
                    };
                    app.loading = true;
                    perform(
                        load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                        Message::BooksLoaded,
                    )
                }
                Err(error) => {
                    app.status = Some(error);
                    Task::none()
                }
            }
        }
        Message::ReadBook => {
            let Page::Detail(book) = &app.page else {
                return Task::none();
            };
            perform(
                open_book(book.current_path.clone(), app.reader_app_id.clone()),
                Message::BookOpened,
            )
        }
        Message::BookOpened(result) => {
            if let Err(error) = result {
                app.status = Some(error);
            }
            Task::none()
        }
        Message::EditBook => {
            let Page::Detail(book) = &app.page else {
                return Task::none();
            };
            perform(
                edit_book(book.current_path.clone(), app.editor_app_id.clone()),
                Message::BookEdited,
            )
        }
        Message::BookEdited(result) => {
            if let Err(error) = result {
                app.status = Some(error);
            }
            Task::none()
        }
        Message::QuickAction(action) => {
            app.status = Some(format!("Acción rápida: {action}."));
            Task::none()
        }
        Message::CardDesignSelected(id) => {
            if card_design(&id).id() != id {
                app.status = Some(format!("El diseño de tarjeta «{id}» no existe."));
                return Task::none();
            }
            app.selected_card_design = id;
            app.status = match save_card_design_preference(&app.selected_card_design) {
                Ok(()) => Some("Diseño de biblioteca actualizado.".into()),
                Err(error) => Some(error),
            };
            Task::none()
        }
        Message::PageSizeSelected(size) => {
            let size = size.clamp(
                *COVERS_PER_PAGE_OPTIONS
                    .first()
                    .unwrap_or(&DEFAULT_COVERS_PER_PAGE),
                *COVERS_PER_PAGE_OPTIONS
                    .last()
                    .unwrap_or(&DEFAULT_COVERS_PER_PAGE),
            );
            app.covers_per_page = size;
            app.status = match save_page_size_preference(size) {
                Ok(()) => Some(format!("Mostrando {size} carátulas por página.").into()),
                Err(error) => Some(error),
            };
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            perform(
                load_books(db, normalized_query(&app.search), 0, app.covers_per_page),
                Message::BooksLoaded,
            )
        }
        Message::LibraryPageSelected(page) => {
            let page_count = app.total_books_count.div_ceil(app.covers_per_page).max(1);
            let target_page = page.min(page_count - 1);
            app.hovered_card = None;
            let Some(db) = app.db.clone() else {
                return Task::none();
            };
            app.loading = true;
            perform(
                load_books(
                    db,
                    normalized_query(&app.search),
                    target_page,
                    app.covers_per_page,
                ),
                Message::BooksLoaded,
            )
        }
        Message::CardHoverChanged(card) => {
            app.hovered_card = card;
            Task::none()
        }
        Message::EnableOpdsServer => {
            if app.opds_server_enabled {
                app.status = Some(opds_browser_status());
                return Task::none();
            }
            let Some(db) = app.db.clone() else {
                app.status = Some("La base de datos todavía no está disponible.".into());
                return Task::none();
            };
            app.status = Some("Iniciando servidor web…".into());
            perform(start_opds_server(db), Message::OpdsServerEnabled)
        }
        Message::OpdsServerEnabled(result) => {
            match result {
                Ok(status) => {
                    app.opds_server_enabled = true;
                    app.status = Some(status);
                }
                Err(error) => app.status = Some(error),
            }
            Task::none()
        }
        Message::ReaderAppSelected(app_id) => {
            app.reader_app_id = app_id;
            app.status = match save_app_preference(AppRole::Reader, app.reader_app_id.as_deref()) {
                Ok(()) => Some("Aplicación de lectura actualizada.".into()),
                Err(error) => Some(error),
            };
            Task::none()
        }
        Message::EditorAppSelected(app_id) => {
            app.editor_app_id = app_id;
            app.status = match save_app_preference(AppRole::Editor, app.editor_app_id.as_deref()) {
                Ok(()) => Some("Aplicación de edición actualizada.".into()),
                Err(error) => Some(error),
            };
            Task::none()
        }
    }
}

fn view_app(app: &App) -> Element<'_, Message> {
    match &app.page {
        Page::Main => main_page(app),
        Page::Detail(book) => detail_page(app, book),
    }
}

fn main_page(app: &App) -> Element<'_, Message> {
    let body = match app.section {
        Section::Library => grid_view(app),
        Section::List => list_view(app),
        Section::Stats => stats_view(app),
        Section::Settings => settings_view(app),
    };

    let mut content = column![body].padding(20);

    if app.loading {
        content = content.push(text("Cargando…"));
    }
    if let Some(status) = &app.status {
        content = content.push(text(status));
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn grid_view(app: &App) -> Element<'_, Message> {
    if app.books.is_empty() {
        let (title, description) = if app.loading {
            ("Cargando biblioteca…", "Preparando libros y portadas.")
        } else {
            (
                "Biblioteca vacía",
                "Importá uno o más archivos EPUB para comenzar.",
            )
        };
        return center(
            column![text(title).size(28), text(description),]
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .height(Length::Fill)
        .into();
    }

    let design = card_design(&app.selected_card_design);
    let page_count = app.total_books_count.div_ceil(app.covers_per_page).max(1);
    let page = app.library_page.min(page_count - 1);
    let visible_books = &app.books;
    let mut grid = column![].spacing(16);
    for books in visible_books.chunks(GRID_COLUMNS) {
        let mut line = row![].spacing(14);
        for book in books {
            line = line.push(
                container(interactive_card(book, design, app.hovered_card))
                    .width(Length::FillPortion(1))
                    .height(CARD_SLOT_HEIGHT),
            );
        }
        for _ in books.len()..GRID_COLUMNS {
            line = line.push(container("").width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }

    column![
        row![
            text("Biblioteca").size(24).font(cosmic::font::bold()),
            text(format!(
                "{} libro(s) · diseño {}",
                app.total_books_count,
                design.label()
            ))
            .size(13),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        scrollable(grid).height(Length::Fill),
        row![
            button::standard("← Anterior")
                .on_press_maybe(page.checked_sub(1).map(Message::LibraryPageSelected)),
            text(format!("Página {} de {page_count}", page + 1))
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            button::standard("Siguiente →").on_press_maybe(
                page.checked_add(1)
                    .filter(|next| *next < page_count)
                    .map(Message::LibraryPageSelected),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}

#[derive(Debug, Clone, Copy)]
enum CardVariant {
    Basic,
    Elevated,
    Gradient,
    Minimal,
    Badges,
    Horizontal,
    Highlighted,
    Progress,
    SecondaryInfo,
    Glass,
    QuickActions,
    Square,
}

impl CardVariant {
    fn label(self) -> &'static str {
        match self {
            Self::Basic => "Básica",
            Self::Elevated => "Elevada",
            Self::Gradient => "Con gradiente",
            Self::Minimal => "Minimal con íconos",
            Self::Badges => "Con etiquetas",
            Self::Horizontal => "Compacta horizontal",
            Self::Highlighted => "Borde resaltado",
            Self::Progress => "Con progreso",
            Self::SecondaryInfo => "Información secundaria",
            Self::Glass => "Vidrio",
            Self::QuickActions => "Acción rápida",
            Self::Square => "Tile cuadrada",
        }
    }
}

/// Contrato que comparte cualquier diseño de tarjeta de libro.
///
/// Para añadir un diseño nuevo se implementa este trait y se incorpora una
/// instancia a `CARD_DESIGNS`. Biblioteca y Configuración consumirán la misma
/// implementación, evitando previews que difieran del resultado real.
pub trait BookCardDesign: Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn render<'a>(&self, book: &'a UiBook) -> Element<'a, Message>;
}

struct BuiltInCardDesign {
    id: &'static str,
    description: &'static str,
    variant: CardVariant,
}

impl BookCardDesign for BuiltInCardDesign {
    fn id(&self) -> &'static str {
        self.id
    }

    fn label(&self) -> &'static str {
        self.variant.label()
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn render<'a>(&self, book: &'a UiBook) -> Element<'a, Message> {
        variant_card(book, self.variant)
    }
}

static CARD_DESIGNS: [&dyn BookCardDesign; SHOWCASE_CARDS] = [
    &BuiltInCardDesign {
        id: "basic",
        description: "Portada, autor y colección con jerarquía clásica.",
        variant: CardVariant::Basic,
    },
    &BuiltInCardDesign {
        id: "elevated",
        description: "Separación visual mediante sombra y badge de serie.",
        variant: CardVariant::Elevated,
    },
    &BuiltInCardDesign {
        id: "gradient",
        description: "Información superpuesta sobre la parte baja de la portada.",
        variant: CardVariant::Gradient,
    },
    &BuiltInCardDesign {
        id: "minimal",
        description: "Tema claro con metadatos e iconografía compacta.",
        variant: CardVariant::Minimal,
    },
    &BuiltInCardDesign {
        id: "badges",
        description: "Etiquetas visibles para categorías y favoritos.",
        variant: CardVariant::Badges,
    },
    &BuiltInCardDesign {
        id: "horizontal",
        description: "Portada pequeña y datos distribuidos horizontalmente.",
        variant: CardVariant::Horizontal,
    },
    &BuiltInCardDesign {
        id: "highlighted",
        description: "Borde de acento para selección o foco destacado.",
        variant: CardVariant::Highlighted,
    },
    &BuiltInCardDesign {
        id: "progress",
        description: "Barra y porcentaje de avance de lectura.",
        variant: CardVariant::Progress,
    },
    &BuiltInCardDesign {
        id: "secondary-info",
        description: "Puntuación, cantidad de páginas y formato.",
        variant: CardVariant::SecondaryInfo,
    },
    &BuiltInCardDesign {
        id: "glass",
        description: "Superficie semitransparente con borde tenue.",
        variant: CardVariant::Glass,
    },
    &BuiltInCardDesign {
        id: "quick-actions",
        description: "Accesos directos para vista, favorito y compartir.",
        variant: CardVariant::QuickActions,
    },
    &BuiltInCardDesign {
        id: "square",
        description: "Composición simétrica centrada en la portada.",
        variant: CardVariant::Square,
    },
];

pub fn card_designs() -> impl Iterator<Item = &'static dyn BookCardDesign> {
    CARD_DESIGNS.iter().copied()
}

fn card_design(id: &str) -> &'static dyn BookCardDesign {
    card_designs()
        .find(|design| design.id() == id)
        .unwrap_or(CARD_DESIGNS[0])
}

fn interactive_card<'a>(
    book: &'a UiBook,
    design: &'static dyn BookCardDesign,
    hovered_card: Option<(i64, &'static str)>,
) -> Element<'a, Message> {
    let card_key = (book.id, design.id());
    let hovered = hovered_card == Some(card_key);
    let visual = container(design.render(book))
        .width(Length::Fill)
        .height(CARD_HEIGHT)
        .style(move |theme| hover_shadow_style(theme, hovered));
    let visual: Element<'a, Message> = if hovered {
        stack![
            visual,
            container("")
                .width(Length::Fill)
                .height(CARD_HEIGHT)
                .style(hover_border_style),
        ]
        .into()
    } else {
        visual.into()
    };

    let positioned: Element<'a, Message> = if hovered {
        column![visual, container("").height(2)]
            .height(CARD_SLOT_HEIGHT)
            .into()
    } else {
        column![container("").height(2), visual]
            .height(CARD_SLOT_HEIGHT)
            .into()
    };

    mouse_area(positioned)
        .on_enter(Message::CardHoverChanged(Some(card_key)))
        .on_exit(Message::CardHoverChanged(None))
        .on_press(Message::OpenDetail(book.id))
        .interaction(cosmic::iced::mouse::Interaction::Pointer)
        .into()
}

fn variant_card<'a>(book: &'a UiBook, variant: CardVariant) -> Element<'a, Message> {
    let series = book.series_name.as_deref().unwrap_or("Sin colección");
    let title = || text(&book.title).size(16).font(cosmic::font::semibold());
    let author = || text(&book.author_name).size(12);
    let standard_cover = || cover(book, Length::Fill, 154.0, 10.0);

    let content: Element<'a, Message> = match variant {
        CardVariant::Basic => column![
            cover_with_action(book, "•••"),
            title(),
            author(),
            row![text("●").size(10), text(series).size(11)]
                .spacing(6)
                .align_y(Alignment::Center),
        ]
        .spacing(7)
        .into(),
        CardVariant::Elevated => column![
            standard_cover(),
            title(),
            author(),
            badge(series, BadgeKind::Neutral),
        ]
        .spacing(7)
        .into(),
        CardVariant::Gradient => column![
            stack![
                cover(book, Length::Fill, 186.0, 10.0),
                container(column![title(), author()].spacing(3).width(Length::Fill))
                    .padding(10)
                    .align_bottom(Length::Fixed(186.0))
                    .style(gradient_caption_style),
            ],
            row![text("●").size(9), text(series).size(11)].spacing(5),
        ]
        .spacing(8)
        .into(),
        CardVariant::Minimal => column![
            cover_with_action(book, "🔖"),
            title(),
            author(),
            divider::horizontal::default(),
            text(format!("▣  {series}")).size(11),
            text("▱  Serie 2").size(11),
            text("□  26 jun 2026").size(11),
        ]
        .spacing(6)
        .into(),
        CardVariant::Badges => column![
            standard_cover(),
            title(),
            author(),
            row![
                badge("Saga", BadgeKind::Green),
                badge("Favorito", BadgeKind::Violet),
            ]
            .spacing(5),
            text(series).size(11),
        ]
        .spacing(7)
        .into(),
        CardVariant::Horizontal => column![
            row![
                cover(book, Length::Fixed(76.0), 116.0, 8.0),
                column![
                    title(),
                    author(),
                    badge(series, BadgeKind::Neutral),
                    text("EPUB · Biblioteca").size(10),
                ]
                .spacing(7)
                .width(Length::Fill),
            ]
            .spacing(10),
            divider::horizontal::default(),
            text("Diseñada para listas compactas").size(11),
        ]
        .spacing(10)
        .into(),
        CardVariant::Highlighted => column![
            cover_with_action(book, "•••"),
            title(),
            author(),
            row![text("●").size(10), text(series).size(11)].spacing(6),
        ]
        .spacing(7)
        .into(),
        CardVariant::Progress => column![
            standard_cover(),
            title(),
            author(),
            row![
                cosmic::widget::determinate_linear(0.65)
                    .width(Length::Fill)
                    .girth(6),
                text("65%").size(11),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(series).size(11),
        ]
        .spacing(7)
        .into(),
        CardVariant::SecondaryInfo => column![
            standard_cover(),
            title(),
            author(),
            text("★★★★☆  4,2").size(13),
            row![
                badge("512 pág.", BadgeKind::Outline),
                badge("EPUB", BadgeKind::Outline),
            ]
            .spacing(5),
        ]
        .spacing(7)
        .into(),
        CardVariant::Glass => column![
            cover_with_action(book, "◇"),
            title(),
            author(),
            divider::horizontal::default(),
            text(format!("Transparencia · {series}")).size(11),
        ]
        .spacing(8)
        .into(),
        CardVariant::QuickActions => column![
            standard_cover(),
            title(),
            author(),
            divider::horizontal::default(),
            row![
                button::standard("◉").on_press(Message::QuickAction("vista previa")),
                button::standard("☆").on_press(Message::QuickAction("favorito")),
                button::standard("↗").on_press(Message::QuickAction("compartir")),
                button::suggested("➜").on_press(Message::OpenDetail(book.id)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(7)
        .into(),
        CardVariant::Square => column![
            cover(book, Length::Fill, 194.0, 10.0),
            title(),
            row![author(), text("·").size(11), text("EPUB").size(10)].spacing(4),
        ]
        .spacing(7)
        .into(),
    };

    let style = match variant {
        CardVariant::Elevated => elevated_card_style,
        CardVariant::Minimal => minimal_card_style,
        CardVariant::Highlighted => highlighted_card_style,
        CardVariant::Glass => glass_card_style,
        _ => standard_card_style,
    };

    container(content)
        .padding(12)
        .width(Length::Fill)
        .height(CARD_HEIGHT)
        .clip(true)
        .style(style)
        .into()
}

fn cover<'a>(book: &UiBook, width: Length, height: f32, radius: f32) -> Element<'a, Message> {
    if let Some(handle) = &book.cover_image {
        cosmic::iced::widget::image(handle.clone())
            .width(width)
            .height(height)
            .content_fit(ContentFit::Cover)
            .border_radius(radius)
            .into()
    } else {
        let placeholder_text = if book.cover_loading {
            "Cargando portada…"
        } else {
            "Sin portada"
        };
        container(
            column![text("▤").size(52), text(placeholder_text).size(11)]
                .spacing(6)
                .align_x(Alignment::Center),
        )
        .center_x(width)
        .center_y(Length::Fixed(height))
        .style(cover_placeholder_style)
        .into()
    }
}

fn cover_with_action<'a>(book: &UiBook, action: &'static str) -> Element<'a, Message> {
    stack![
        cover(book, Length::Fill, 154.0, 10.0),
        container(text(action).size(15).font(cosmic::font::bold()))
            .padding([6, 9])
            .align_right(Length::Fill)
            .align_top(Length::Fixed(154.0)),
    ]
    .into()
}

#[derive(Clone, Copy)]
enum BadgeKind {
    Neutral,
    Green,
    Violet,
    Outline,
}

fn badge<'a>(label: &'a str, kind: BadgeKind) -> Element<'a, Message> {
    container(text(label).size(10))
        .padding([3, 7])
        .style(move |_| badge_style(kind))
        .into()
}

fn badge_style(kind: BadgeKind) -> cosmic::iced::widget::container::Style {
    let (background, text_color, border_color) = match kind {
        BadgeKind::Neutral => (
            Color::from_rgba8(100, 110, 125, 0.28),
            Color::WHITE,
            Color::TRANSPARENT,
        ),
        BadgeKind::Green => (
            Color::from_rgba8(46, 160, 100, 0.32),
            Color::from_rgb8(190, 245, 215),
            Color::TRANSPARENT,
        ),
        BadgeKind::Violet => (
            Color::from_rgba8(135, 85, 210, 0.34),
            Color::from_rgb8(229, 210, 255),
            Color::TRANSPARENT,
        ),
        BadgeKind::Outline => (
            Color::TRANSPARENT,
            Color::WHITE,
            Color::from_rgba8(190, 195, 210, 0.6),
        ),
    };
    cosmic::iced::widget::container::Style {
        text_color: Some(text_color),
        background: Some(Background::Color(background)),
        border: Border {
            color: border_color,
            width: if matches!(kind, BadgeKind::Outline) {
                1.0
            } else {
                0.0
            },
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn card_legend() -> Element<'static, Message> {
    container(
        row![
            legend_item("•••", "Menú"),
            legend_item("🔖", "Marcador"),
            legend_item("▣", "Colección"),
            legend_item("▱", "Serie"),
            legend_item("□", "Fecha"),
            legend_item("☆", "Favorito"),
            legend_item("━", "Progreso"),
            legend_item("➜", "Acción rápida"),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([10, 14])
    .width(Length::Fill)
    .style(legend_style)
    .into()
}

fn legend_item(icon: &'static str, label: &'static str) -> Element<'static, Message> {
    row![
        text(icon).size(14).font(cosmic::font::semibold()),
        text(label).size(11),
    ]
    .spacing(5)
    .align_y(Alignment::Center)
    .into()
}

fn standard_card_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let cosmic = theme.cosmic();
    cosmic::iced::widget::container::Style {
        text_color: Some(cosmic.background.component.on.into()),
        background: Some(Background::Color(cosmic.background.component.base.into())),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn hover_border_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        border: Border {
            color: theme.cosmic().background.divider.into(),
            width: 2.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

fn hover_shadow_style(
    theme: &cosmic::Theme,
    hovered: bool,
) -> cosmic::iced::widget::container::Style {
    if !hovered {
        return Default::default();
    }
    cosmic::iced::widget::container::Style {
        shadow: Shadow {
            color: theme.cosmic().shade.into(),
            offset: Vector::new(0.0, 5.0),
            blur_radius: 18.0,
        },
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn elevated_card_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.38),
            offset: Vector::new(0.0, 5.0),
            blur_radius: 14.0,
        },
        ..standard_card_style(theme)
    }
}

fn highlighted_card_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let mut style = standard_card_style(theme);
    style.border.color = theme.cosmic().accent.base.into();
    style.border.width = 3.0;
    style
}

fn minimal_card_style(_: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        text_color: Some(Color::from_rgb8(52, 45, 38)),
        background: Some(Background::Color(Color::from_rgb8(239, 229, 207))),
        border: Border {
            color: Color::from_rgb8(213, 197, 169),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

fn glass_card_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let mut background: Color = theme.cosmic().background.component.base.into();
    background.a = 0.68;
    cosmic::iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: Color::from_rgba8(255, 255, 255, 0.22),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..standard_card_style(theme)
    }
}

fn cover_placeholder_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let cosmic = theme.cosmic();
    cosmic::iced::widget::container::Style {
        text_color: Some(cosmic.background.component.on.into()),
        background: Some(Background::Color(cosmic.background.component.hover.into())),
        border: Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn gradient_caption_style(_: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgba8(8, 10, 15, 0.78))),
        border: Border {
            radius: [0.0, 0.0, 10.0, 10.0].into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn legend_style(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let cosmic = theme.cosmic();
    cosmic::iced::widget::container::Style {
        text_color: Some(cosmic.background.component.on.into()),
        background: Some(Background::Color(cosmic.background.component.base.into())),
        border: Border {
            color: cosmic.background.divider.into(),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn list_view(app: &App) -> Element<'_, Message> {
    let mut list = column![
        row![
            text("Título / Autor").width(Length::FillPortion(4)),
            text("Saga").width(Length::FillPortion(2)),
            text("Salud").width(Length::FillPortion(1)),
            text("Ubicación").width(Length::FillPortion(1)),
        ]
        .spacing(12),
        divider::horizontal::default()
    ]
    .spacing(8);

    for book in &app.books {
        let health = if std::path::Path::new(&book.current_path).exists() {
            "✓ OK"
        } else {
            "✗ No encontrado"
        };
        let location = if book.is_normalized {
            "Normalizado"
        } else {
            "Original"
        };
        let title = column![text(&book.title), text(&book.author_name).size(13)].spacing(2);
        let item = row![
            title.width(Length::FillPortion(4)),
            text(book.series_name.as_deref().unwrap_or("—")).width(Length::FillPortion(2)),
            text(health).width(Length::FillPortion(1)),
            text(location).width(Length::FillPortion(1)),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        list = list.push(
            button::custom(item)
                .on_press(Message::OpenDetail(book.id))
                .width(Length::Fill),
        );
    }

    scrollable(list).height(Length::Fill).into()
}

fn stats_view(app: &App) -> Element<'_, Message> {
    let minutes = app.stats.reading_time_secs / 60;
    let reading_time = if minutes >= 60 {
        format!("{} h {} min", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    };

    scrollable(
        column![
            text("Estadísticas de la biblioteca").size(28),
            metric("Total de libros", app.stats.total_books),
            metric_text("Tiempo total de lectura", reading_time),
            metric("Autores", app.stats.authors),
            metric("Sagas / Series", app.stats.series),
            metric("Etiquetas", app.stats.tags),
        ]
        .spacing(14)
        .max_width(720),
    )
    .height(Length::Fill)
    .into()
}

fn metric(label: &'static str, value: impl ToString) -> Element<'static, Message> {
    metric_text(label, value.to_string())
}

fn metric_text(label: &'static str, value: String) -> Element<'static, Message> {
    container(
        row![text(label).width(Length::Fill), text(value).size(20)].align_y(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .into()
}

fn settings_view(app: &App) -> Element<'_, Message> {
    let library_path = scriptorium::default_db_path()
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "No disponible".into());

    let picker: Element<'_, Message> = if let Some(preview_book) = app.books.first() {
        card_design_picker(app, preview_book)
    } else {
        container(
            column![
                text("No hay libros para previsualizar").size(18),
                text("Importá un EPUB y aquí podrás comparar todos los diseños."),
            ]
            .spacing(6),
        )
        .padding(16)
        .style(standard_card_style)
        .into()
    };

    scrollable(
        column![
            text("Configuración").size(28),
            text("Carátulas por página").size(20).font(cosmic::font::bold()),
            text("Cantidad de libros que se muestran en cada página de la biblioteca."),
            page_size_picker(app),
            divider::horizontal::default(),
            text("Aplicaciones").size(20).font(cosmic::font::bold()),
            text("Elegí qué aplicación abre los EPUB para lectura y cuál se usa para edición."),
            app_selector(
                "Leer EPUB",
                &app.epub_apps,
                app.reader_app_id.as_deref(),
                Message::ReaderAppSelected,
            ),
            app_selector(
                "Editar EPUB",
                &app.epub_apps,
                app.editor_app_id.as_deref(),
                Message::EditorAppSelected,
            ),
            divider::horizontal::default(),
            text("Diseño de las tarjetas").size(20).font(cosmic::font::bold()),
            text("Elegí una opción. La misma implementación se usa en esta vista previa y en Biblioteca."),
            picker,
            card_legend(),
            divider::horizontal::default(),
            text("Directorio de la biblioteca").size(18),
            text(library_path),
            divider::horizontal::default(),
            text("Servidor web").size(20).font(cosmic::font::bold()),
            text("Activa una vista web local de la biblioteca y el catálogo OPDS para lectores compatibles."),
            opds_server_controls(app),
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}

fn app_selector<'a>(
    title: &'static str,
    apps: &'a [SystemApp],
    selected_id: Option<&'a str>,
    message: fn(Option<String>) -> Message,
) -> Element<'a, Message> {
    let selected_label = selected_id
        .and_then(|id| apps.iter().find(|app| app.id == id))
        .map(|app| app.name.as_str())
        .or(selected_id)
        .unwrap_or("Predeterminada del sistema");

    let mut options = column![
        row![
            text(title).size(16).font(cosmic::font::semibold()),
            text(selected_label).size(14).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        app_option_button(
            "Predeterminada del sistema",
            selected_id.is_none(),
            message(None),
        ),
    ]
    .spacing(8);

    if apps.is_empty() {
        options =
            options.push(text("No se encontraron aplicaciones registradas para EPUB.").size(13));
    } else {
        for app_row in apps.chunks(3) {
            let mut row_widgets = row![].spacing(8);
            for app in app_row {
                row_widgets = row_widgets.push(app_option_button(
                    app.name.clone(),
                    selected_id == Some(app.id.as_str()),
                    message(Some(app.id.clone())),
                ));
            }
            for _ in app_row.len()..3 {
                row_widgets = row_widgets.push(container("").width(Length::FillPortion(1)));
            }
            options = options.push(row_widgets);
        }
    }

    container(options)
        .padding(16)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card)
        .into()
}

fn app_option_button<'a>(
    label: impl Into<String>,
    selected: bool,
    message: Message,
) -> Element<'a, Message> {
    let button = if selected {
        button::suggested(label.into())
    } else {
        button::standard(label.into()).on_press(message)
    };
    button.width(Length::FillPortion(1)).into()
}

fn opds_server_controls(app: &App) -> Element<'_, Message> {
    let browser_url = opds_browser_url();
    let opds_url = opds_catalog_url();
    let action: Element<'_, Message> = if app.opds_server_enabled {
        button::suggested("Servidor activo").into()
    } else {
        button::standard("Habilitar servidor web")
            .on_press(Message::EnableOpdsServer)
            .into()
    };

    container(
        column![
            row![
                action,
                column![text(browser_url).size(14), text(opds_url).size(13),]
                    .spacing(2)
                    .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            text(if app.opds_server_enabled {
                "Disponible hasta cerrar Scriptorium."
            } else {
                "El servidor escucha en la red local al activarse."
            })
            .size(13),
        ]
        .spacing(8),
    )
    .padding(16)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card)
    .into()
}

fn page_size_picker(app: &App) -> Element<'_, Message> {
    let mut line = row![].spacing(8);
    for size in COVERS_PER_PAGE_OPTIONS {
        let selected = size == app.covers_per_page;
        let widget: Element<'_, Message> = if selected {
            button::suggested(size.to_string()).into()
        } else {
            button::standard(size.to_string())
                .on_press(Message::PageSizeSelected(size))
                .into()
        };
        line = line.push(widget);
    }
    line.into()
}

fn card_design_picker<'a>(app: &'a App, preview_book: &'a UiBook) -> Element<'a, Message> {
    let designs: Vec<_> = card_designs().collect();
    let mut grid = column![].spacing(16);

    for design_row in designs.chunks(GRID_COLUMNS) {
        let mut line = row![].spacing(14);
        for design in design_row {
            let selected = design.id() == app.selected_card_design;
            let selector: Element<'_, Message> = if selected {
                button::suggested("Seleccionada").into()
            } else {
                button::standard("Usar este diseño")
                    .on_press(Message::CardDesignSelected(design.id().to_string()))
                    .into()
            };

            line = line.push(
                column![
                    row![
                        text(design.label()).size(13).font(cosmic::font::semibold()),
                        if selected {
                            text("● Activa").size(11)
                        } else {
                            text("").size(11)
                        },
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    text(design.description()).size(10).height(32),
                    interactive_card(preview_book, *design, app.hovered_card),
                    selector,
                ]
                .spacing(7)
                .width(Length::FillPortion(1)),
            );
        }
        for _ in design_row.len()..GRID_COLUMNS {
            line = line.push(container("").width(Length::FillPortion(1)));
        }
        grid = grid.push(line);
    }

    grid.into()
}

fn detail_page<'a>(app: &'a App, book: &'a BookDetail) -> Element<'a, Message> {
    let cover_image = app
        .books
        .iter()
        .find(|b| b.id == book.id)
        .and_then(|b| b.cover_image.clone());

    let cover_widget: Element<'a, Message> = if let Some(ref handle) = cover_image {
        cosmic::iced::widget::image(handle.clone())
            .width(210)
            .height(300)
            .content_fit(ContentFit::Cover)
            .border_radius(12.0)
            .into()
    } else {
        container(
            column![text("▤").size(64), text("TODO: portada").size(14)]
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .center_x(Length::Fixed(210.0))
        .center_y(Length::Fixed(300.0))
        .class(cosmic::theme::Container::Secondary)
        .into()
    };

    let cover_with_edition = stack![
        cover_widget,
        container(text("✓ TODO: edición").size(13))
            .padding([5, 8])
            .align_right(Length::Fixed(210.0))
            .align_bottom(Length::Fixed(300.0))
            .class(cosmic::theme::Container::Secondary),
    ];

    let file_format = std::path::Path::new(&book.current_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_uppercase)
        .unwrap_or_else(|| "TODO".into());
    let file_size = std::fs::metadata(&book.current_path)
        .map(|metadata| human_file_size(metadata.len()))
        .unwrap_or_else(|_| "TODO: calcular".into());
    let path_preview = middle_ellipsis(&book.current_path, 78);
    let collection = book.series_name.as_deref().unwrap_or("Sin colección");

    let identity = container(
        row![
            cover_with_edition,
            column![
                text(&book.title).size(34).font(cosmic::font::bold()),
                text(&book.author_name).size(20),
                row![
                    semantic_tag(collection.to_string()),
                    semantic_tag("TODO: género".to_string()),
                ]
                .spacing(8),
                row![text("☆☆☆☆☆").size(20), text("TODO: puntuación").size(14)]
                    .spacing(8)
                    .align_y(Alignment::Center),
                divider::horizontal::default(),
                row![
                    column![
                        detail_value("Añadido el", book.date_added.format("%d/%m/%Y").to_string()),
                        detail_value("Formato", file_format),
                    ]
                    .spacing(8)
                    .width(Length::FillPortion(1)),
                    column![
                        detail_value("Tamaño", file_size),
                        detail_value("Ruta", path_preview),
                    ]
                    .spacing(8)
                    .width(Length::FillPortion(2)),
                ]
                .spacing(20),
            ]
            .spacing(14)
            .width(Length::Fill),
        ]
        .spacing(24)
        .align_y(Alignment::Start),
    )
    .padding(18)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    let tabs = row![
        button::suggested("Información").on_press(Message::QuickAction("pestaña Información")),
        button::standard("Capítulos").on_press(Message::QuickAction("TODO: cargar capítulos")),
        button::standard("Notas").on_press(Message::QuickAction("TODO: cargar notas")),
        button::standard("Marcas").on_press(Message::QuickAction("TODO: cargar marcas")),
        button::standard("Historial").on_press(Message::QuickAction("TODO: cargar historial")),
    ]
    .spacing(6);

    let technical_details = container(
        column![
            row![
                text("Detalles técnicos")
                    .size(18)
                    .font(cosmic::font::bold())
                    .width(Length::Fill),
                button::standard("Editar").on_press(Message::OpenEditDialog),
                button::destructive("Eliminar").on_press(Message::OpenDeleteDialog),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                column![
                    detail_value("Título", book.title.clone()),
                    detail_value("Autor", book.author_name.clone()),
                    detail_value("Idioma", "TODO: extraer del EPUB".into()),
                    detail_value("Editorial", "TODO: extraer del EPUB".into()),
                ]
                .spacing(10)
                .width(Length::Fill),
                column![
                    detail_value("Serie", collection.to_string()),
                    detail_value("Volumen", "TODO".into()),
                    detail_value("ISBN", "TODO: extraer del EPUB".into()),
                    detail_value("Publicación", "TODO: extraer del EPUB".into()),
                ]
                .spacing(10)
                .width(Length::Fill),
            ]
            .spacing(20),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::FillPortion(2))
    .class(cosmic::theme::Container::Card);

    let reading_progress = container(
        column![
            text("Progreso de lectura")
                .size(18)
                .font(cosmic::font::bold()),
            stack![
                cosmic::widget::determinate_circular(0.0)
                    .size(112.0)
                    .bar_height(8.0),
                center(text("0%").size(24).font(cosmic::font::bold()))
                    .width(112)
                    .height(112),
            ],
            text("TODO: conectar el progreso almacenado del libro.").size(14),
            text("Aún no has comenzado a leer este libro.").size(14),
            row![
                button::suggested("Comenzar a leer").on_press(Message::ReadBook),
                button::standard("Editar EPUB").on_press(Message::EditBook),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .padding(16)
    .width(Length::FillPortion(1))
    .class(cosmic::theme::Container::Card);

    let information = row![technical_details, reading_progress]
        .spacing(14)
        .width(Length::Fill);

    let description_card = container(
        column![
            text("Descripción").size(18).font(cosmic::font::bold()),
            text("TODO: cargar la sinopsis desde los metadatos del EPUB o la base de datos.")
                .size(15),
            button::standard("Ver más ↓")
                .on_press(Message::QuickAction("TODO: expandir descripción")),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    let collections_card = container(
        column![
            text("Colecciones").size(18).font(cosmic::font::bold()),
            text(if book.series_name.is_some() {
                "Serie detectada; TODO: integrar colecciones editables."
            } else {
                "Este libro no pertenece a ninguna colección."
            })
            .size(15),
            button::standard("＋ Añadir a colección")
                .on_press(Message::QuickAction("TODO: añadir a colección")),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    let tags_card = container(
        column![
            text("Etiquetas").size(18).font(cosmic::font::bold()),
            row![
                semantic_tag("TODO: etiquetas".into()),
                semantic_tag("TODO: sincronizar".into()),
            ]
            .spacing(7),
            button::standard("＋ Añadir etiqueta")
                .on_press(Message::QuickAction("TODO: añadir etiqueta")),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    let right_sidebar = column![description_card, collections_card, tags_card]
        .spacing(14)
        .width(Length::FillPortion(1));

    let activity = container(
        row![
            text("◷").size(22),
            column![
                text("Libro añadido a la biblioteca").font(cosmic::font::semibold()),
                text(format!(
                    "{} · TODO: hora exacta",
                    book.date_added.format("%d/%m/%Y")
                ))
                .size(14),
            ]
            .spacing(3)
            .width(Length::Fill),
            text("TODO: tiempo relativo").size(13),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(14)
    .width(Length::Fill)
    .class(cosmic::theme::Container::Card);

    let mut primary_content = column![identity, tabs, information, activity]
        .spacing(14)
        .width(Length::FillPortion(3));

    if app.loading {
        primary_content = primary_content.push(text("Procesando…"));
    }
    if let Some(status) = &app.status {
        primary_content = primary_content.push(text(status));
    }

    let main_columns = row![primary_content, right_sidebar]
        .spacing(16)
        .width(Length::Fill)
        .align_y(Alignment::Start);

    let detail_content = column![main_columns]
        .spacing(14)
        .padding(20)
        .width(Length::Fill);

    scrollable(detail_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn semantic_tag(label: String) -> Element<'static, Message> {
    container(text(label).size(13))
        .padding([4, 8])
        .class(cosmic::theme::Container::Secondary)
        .into()
}

fn detail_value(label: &'static str, value: String) -> Element<'static, Message> {
    column![
        text(label).size(13).font(cosmic::font::semibold()),
        text(value).size(15),
    ]
    .spacing(2)
    .into()
}

fn human_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn middle_ellipsis(value: &str, max_chars: usize) -> String {
    let chars: Vec<_> = value.chars().collect();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    let side = max_chars.saturating_sub(1) / 2;
    format!(
        "{}…{}",
        chars[..side].iter().collect::<String>(),
        chars[chars.len() - side..].iter().collect::<String>()
    )
}

fn normalized_query(query: &str) -> Option<String> {
    let query = query.trim().replace('"', "");
    (!query.is_empty()).then(|| format!("{query}*"))
}

#[derive(Debug, Clone, Copy)]
enum AppRole {
    Reader,
    Editor,
}

impl AppRole {
    fn preference_file(self) -> &'static str {
        match self {
            Self::Reader => "reader-app",
            Self::Editor => "editor-app",
        }
    }
}

fn config_root() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .map(|root| root.join("scriptorium"))
}

fn config_path(file_name: &str) -> Option<PathBuf> {
    Some(config_root()?.join(file_name))
}

fn card_design_preference_path() -> Option<PathBuf> {
    config_path("card-design")
}

fn load_card_design_preference() -> String {
    let Some(path) = card_design_preference_path() else {
        return CARD_DESIGNS[0].id().to_string();
    };
    let Ok(id) = std::fs::read_to_string(path) else {
        return CARD_DESIGNS[0].id().to_string();
    };
    card_design(id.trim()).id().to_string()
}

fn save_card_design_preference(id: &str) -> Result<(), String> {
    let path = card_design_preference_path()
        .ok_or_else(|| "No se encontró un directorio de configuración.".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "La ruta de configuración no es válida.".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear la configuración: {error}"))?;
    std::fs::write(path, id)
        .map_err(|error| format!("No se pudo guardar el diseño de tarjeta: {error}"))
}

fn page_size_preference_path() -> Option<PathBuf> {
    config_path("covers-per-page")
}

fn load_page_size_preference() -> usize {
    let Some(path) = page_size_preference_path() else {
        return DEFAULT_COVERS_PER_PAGE;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return DEFAULT_COVERS_PER_PAGE;
    };
    text.trim()
        .parse::<usize>()
        .ok()
        .filter(|size| COVERS_PER_PAGE_OPTIONS.contains(size))
        .unwrap_or(DEFAULT_COVERS_PER_PAGE)
}

fn save_page_size_preference(size: usize) -> Result<(), String> {
    let path = page_size_preference_path()
        .ok_or_else(|| "No se encontró un directorio de configuración.".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "La ruta de configuración no es válida.".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear la configuración: {error}"))?;
    std::fs::write(path, size.to_string())
        .map_err(|error| format!("No se pudo guardar el tamaño de página: {error}"))
}

fn app_preference_path(role: AppRole) -> Option<PathBuf> {
    config_path(role.preference_file())
}

fn load_app_preference(role: AppRole) -> Option<String> {
    let path = app_preference_path(role)?;
    let id = std::fs::read_to_string(path).ok()?;
    let id = id.trim();
    (!id.is_empty()).then(|| id.to_string())
}

fn save_app_preference(role: AppRole, app_id: Option<&str>) -> Result<(), String> {
    let path = app_preference_path(role)
        .ok_or_else(|| "No se encontró un directorio de configuración.".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "La ruta de configuración no es válida.".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("No se pudo crear la configuración: {error}"))?;
    match app_id {
        Some(app_id) => std::fs::write(path, app_id)
            .map_err(|error| format!("No se pudo guardar la aplicación: {error}")),
        None => match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("No se pudo limpiar la aplicación: {error}")),
        },
    }
}

fn discover_epub_apps() -> Vec<SystemApp> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();

    for dir in application_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            let Some(app) = parse_epub_desktop_app(&path) else {
                continue;
            };
            if seen.insert(app.id.clone()) {
                apps.push(app);
            }
        }
    }

    apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    apps
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME").filter(|path| !path.is_empty()) {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(std::env::split_paths(&data_dirs).map(|path| path.join("applications")));
    dirs
}

fn parse_epub_desktop_app(path: &Path) -> Option<SystemApp> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut mime_types = None;
    let mut hidden = false;
    let mut no_display = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "Name" => name = Some(value.to_string()),
            "MimeType" => mime_types = Some(value.to_string()),
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    if hidden || no_display {
        return None;
    }
    let mime_types = mime_types?;
    if !mime_types
        .split(';')
        .any(|mime| mime == "application/epub+zip")
    {
        return None;
    }

    Some(SystemApp {
        id: path.file_name()?.to_str()?.to_string(),
        name: name?,
    })
}

async fn open_database() -> Result<DbHandle, String> {
    LibraryDb::new(&scriptorium::default_db_url())
        .await
        .map(DbHandle)
        .map_err(|error| format!("No se pudo abrir la base de datos: {error}"))
}

async fn start_opds_server(db: LibraryDb) -> Result<String, String> {
    SyncSubsystem::start_opds_server(db, OPDS_SERVER_PORT)
        .await
        .map_err(|error| format!("No se pudo iniciar el servidor web: {error}"))?;
    Ok(opds_browser_status())
}

fn opds_browser_status() -> String {
    format!("Servidor web activo: {}", opds_browser_url())
}

fn opds_browser_url() -> String {
    format!("http://localhost:{OPDS_SERVER_PORT}/opds/browser")
}

fn opds_catalog_url() -> String {
    format!("OPDS: http://localhost:{OPDS_SERVER_PORT}/opds")
}

fn extract_cover_bytes(epub_path: &str, cover_href: Option<String>) -> Option<Vec<u8>> {
    let core = gutencore::GutenCore::open_epub(epub_path).ok()?;
    let href = match cover_href {
        Some(h) => h,
        None => core.get_cover_image()?.href.clone(),
    };
    let opf_dir = core.opf_dir.as_ref()?;
    let abs = opf_dir.join(&href);
    let bytes = std::fs::read(&abs).ok();
    drop(core);
    bytes
}

fn cover_cache_root() -> Option<PathBuf> {
    let cache_root = std::env::var_os("XDG_CACHE_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .map(|home| PathBuf::from(home).join(".cache"))
        })?;
    Some(cache_root.join("scriptorium").join("covers"))
}

fn cover_cache_key(book: &BookListItem) -> String {
    stable_cover_cache_key(book.file_hash.as_deref(), book.id, &book.current_path)
}

fn stable_cover_cache_key(file_hash: Option<&str>, book_id: i64, current_path: &str) -> String {
    stable_cover_cache_key_with_version(COVER_CACHE_VERSION, file_hash, book_id, current_path)
}

fn stable_cover_cache_key_with_version(
    version: &str,
    file_hash: Option<&str>,
    book_id: i64,
    current_path: &str,
) -> String {
    if let Some(hash) = file_hash.filter(|hash| {
        hash.len() == 64 && hash.bytes().all(|character| character.is_ascii_hexdigit())
    }) {
        return format!("{version}-{}", hash.to_ascii_lowercase());
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file_hash.hash(&mut hasher);
    book_id.hash(&mut hasher);
    current_path.hash(&mut hasher);
    if let Ok(metadata) = std::fs::metadata(current_path) {
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified()
            && let Ok(timestamp) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            timestamp.as_nanos().hash(&mut hasher);
        }
    }
    format!("{version}-fallback-{:016x}", hasher.finish())
}

fn cover_cache_path(key: &str) -> Option<PathBuf> {
    Some(cover_cache_root()?.join(format!("{key}.cover")))
}

async fn read_cached_cover_async(key: &str) -> Option<Option<cosmic::iced::widget::image::Handle>> {
    let path = cover_cache_path(key)?;
    let bytes = tokio::fs::read(path).await.ok()?;
    if bytes.is_empty() {
        Some(None)
    } else {
        Some(Some(cosmic::iced::widget::image::Handle::from_bytes(bytes)))
    }
}

fn read_cached_cover_bytes(key: &str) -> Option<Vec<u8>> {
    std::fs::read(cover_cache_path(key)?).ok()
}

fn migrate_legacy_cached_cover(
    book: &BookListItem,
    new_key: &str,
) -> Option<Option<cosmic::iced::widget::image::Handle>> {
    let legacy_key = stable_cover_cache_key_with_version(
        "v1",
        book.file_hash.as_deref(),
        book.id,
        &book.current_path,
    );
    let legacy_path = cover_cache_path(&legacy_key)?;
    let legacy_bytes = read_cached_cover_bytes(&legacy_key)?;
    let optimized = if legacy_bytes.is_empty() {
        Vec::new()
    } else {
        thumbnail_cover_bytes(&legacy_bytes).unwrap_or(legacy_bytes)
    };

    if write_cached_cover(new_key, &optimized).is_ok() {
        let _ = std::fs::remove_file(legacy_path);
    }
    if optimized.is_empty() {
        Some(None)
    } else {
        Some(Some(cosmic::iced::widget::image::Handle::from_bytes(
            optimized,
        )))
    }
}

fn write_cached_cover(key: &str, bytes: &[u8]) -> std::io::Result<()> {
    let Some(path) = cover_cache_path(key) else {
        return Ok(());
    };
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent)?;

    let temp_id = COVER_CACHE_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let temp_path = parent.join(format!(".{key}.tmp-{}-{temp_id}", std::process::id()));
    std::fs::write(&temp_path, bytes)?;
    match std::fs::rename(&temp_path, &path) {
        Ok(()) => Ok(()),
        Err(_error) if path.exists() => {
            let _ = std::fs::remove_file(temp_path);
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_file(temp_path);
            Err(error)
        }
    }
}

fn extract_and_cache_cover(
    epub_path: &str,
    cover_href: Option<String>,
    cache_key: &str,
) -> Option<cosmic::iced::widget::image::Handle> {
    let bytes = extract_cover_bytes(epub_path, cover_href)
        .map(|bytes| thumbnail_cover_bytes(&bytes).unwrap_or(bytes));
    // Un archivo vacío funciona como caché negativo para libros sin portada.
    let _ = write_cached_cover(cache_key, bytes.as_deref().unwrap_or_default());
    bytes.map(cosmic::iced::widget::image::Handle::from_bytes)
}

fn thumbnail_cover_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(bytes).ok()?;
    if image.width() <= COVER_THUMBNAIL_WIDTH && image.height() <= COVER_THUMBNAIL_HEIGHT {
        return Some(bytes.to_vec());
    }

    let thumbnail = image.thumbnail(COVER_THUMBNAIL_WIDTH, COVER_THUMBNAIL_HEIGHT);
    let mut output = std::io::Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, image::ImageFormat::Png)
        .ok()?;
    Some(output.into_inner())
}

async fn load_books(
    db: LibraryDb,
    query: Option<String>,
    page: usize,
    page_size: usize,
) -> Result<LoadedBooks, String> {
    let limit = page_size as i64;
    let offset = (page * page_size) as i64;

    let (books, total_count) = match query {
        Some(query) => {
            let count = db
                .count_search_fts(&query)
                .await
                .map_err(|error| format!("No se pudo contar la búsqueda: {error}"))?;
            let items = db
                .search_fts_subset(&query, Some(limit), Some(offset))
                .await
                .map_err(|error| format!("No se pudo cargar la búsqueda: {error}"))?;
            (items, count as usize)
        }
        None => {
            let count = db
                .count_books()
                .await
                .map_err(|error| format!("No se pudo contar la biblioteca: {error}"))?;
            let items = db
                .list_books_subset(Some(limit), Some(offset))
                .await
                .map_err(|error| format!("No se pudo cargar la biblioteca: {error}"))?;
            (items, count as usize)
        }
    };

    let ui_books: Vec<UiBook> = books
        .iter()
        .map(|book| UiBook {
            id: book.id,
            title: book.title.clone(),
            author_name: book.author_name.clone(),
            series_name: book.series_name.clone(),
            current_path: book.current_path.clone(),
            is_normalized: book.is_normalized,
            cover_image: None,
            cover_loading: true,
        })
        .collect();

    Ok(LoadedBooks {
        books: ui_books,
        raw_books: books,
        total_count,
        page,
    })
}

async fn load_cover(
    book: BookListItem,
    key: String,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> Result<(i64, String, Option<cosmic::iced::widget::image::Handle>), String> {
    // 1. Intentar leer caché estándar de forma asíncrona (no bloquea el ejecutor principal)
    if let Some(cover) = read_cached_cover_async(&key).await {
        return Ok((book.id, key, cover));
    }

    // 2. Si no está en caché estándar, adquirir permiso para operaciones pesadas de E/S y CPU
    let _permit = semaphore.acquire_owned().await.ok();
    let key_clone = key.clone();
    let epub_path = book.current_path.clone();
    let cover_href = book.cover_href.clone();
    let book_for_migration = book.clone();
    let cover = tokio::task::spawn_blocking(move || {
        // Intentar migrar caché legacy
        if let Some(cover) = migrate_legacy_cached_cover(&book_for_migration, &key_clone) {
            return cover;
        }
        // Extraer portada de EPUB
        extract_and_cache_cover(&epub_path, cover_href, &key_clone)
    })
    .await
    .unwrap_or(None);

    Ok((book.id, key, cover))
}

async fn load_detail(db: LibraryDb, id: i64) -> Result<Option<BookDetail>, String> {
    db.get_book(id)
        .await
        .map_err(|error| format!("No se pudo cargar el libro: {error}"))
}

async fn load_stats(db: LibraryDb) -> Result<Stats, String> {
    let LibraryMetrics {
        total_books,
        total_reading_time_secs,
    } = Analytics::get_global_metrics(&db)
        .await
        .map_err(|error| error.to_string())?;
    let authors = db.list_authors().await.map_err(|error| error.to_string())?;
    let series = db.list_series().await.map_err(|error| error.to_string())?;
    let tags = db.list_tags().await.map_err(|error| error.to_string())?;

    Ok(Stats {
        total_books,
        reading_time_secs: total_reading_time_secs,
        authors: authors.len(),
        series: series.len(),
        tags: tags.len(),
    })
}

async fn select_epubs() -> Result<Vec<PathBuf>, String> {
    let dialog = cosmic::dialog::file_chooser::open::Dialog::new().title("Importar EPUB");

    match dialog.open_files().await {
        Ok(response) => {
            let paths = response
                .urls()
                .iter()
                .filter_map(|url| url.to_file_path().ok())
                .filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("epub"))
                })
                .collect();

            Ok(paths)
        }
        Err(cosmic::dialog::file_chooser::Error::Cancelled) => Ok(Vec::new()),
        Err(error) => Err(format!("No se pudo abrir el selector de archivos: {error}")),
    }
}

async fn import_books(db: LibraryDb, paths: Vec<PathBuf>) -> Result<usize, String> {
    let mut imported = 0;
    let mut errors = Vec::new();

    for path in paths {
        let display = path.display().to_string();
        match Pipeline::import_file(&db, display.clone()).await {
            Ok(ImportStatus::Imported) => imported += 1,
            Ok(ImportStatus::Duplicate { .. }) => {}
            Err(error) => errors.push(format!("{display}: {error}")),
        }
    }

    if errors.is_empty() || imported > 0 {
        Ok(imported)
    } else {
        Err(format!("No se pudo importar:\n{}", errors.join("\n")))
    }
}

async fn save_metadata(
    db: LibraryDb,
    id: i64,
    title: String,
    author: String,
) -> Result<(), String> {
    let title = title.trim();
    let author = author.trim();
    if title.is_empty() || author.is_empty() {
        return Err("Título y autor no pueden quedar vacíos.".into());
    }
    db.update_book(id, Some(title), None, Some(author), None)
        .await
        .map_err(|error| format!("No se pudieron guardar los metadatos: {error}"))
}

async fn delete_book(
    db: LibraryDb,
    book_id: i64,
    delete_file: bool,
) -> Result<Option<String>, String> {
    db.delete_book(book_id, delete_file)
        .await
        .map_err(|error| format!("No se pudo eliminar el libro: {error}"))
}

async fn open_book(path: String, app_id: Option<String>) -> Result<(), String> {
    if let Some(app_id) = app_id {
        return launch_desktop_app(&app_id, &path)
            .or_else(|_| launch_with_xdg_open(&path, "abrir"));
    }
    launch_with_xdg_open(&path, "abrir")
}

fn launch_with_xdg_open(path: &str, action: &str) -> Result<(), String> {
    let status = std::process::Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|error| format!("No se pudo {action} {path}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("La aplicación predeterminada rechazó {path}."))
    }
}

fn launch_desktop_app(app_id: &str, path: &str) -> Result<(), String> {
    let status = std::process::Command::new("gtk-launch")
        .arg(app_id)
        .arg(path)
        .status()
        .map_err(|error| format!("No se pudo iniciar {app_id}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{app_id} rechazó {path}."))
    }
}

async fn edit_book(path: String, app_id: Option<String>) -> Result<(), String> {
    if let Some(app_id) = app_id {
        return launch_desktop_app(&app_id, &path)
            .or_else(|_| launch_with_xdg_open(&path, "editar"));
    }

    let status = std::process::Command::new("rubrica-cosmic")
        .arg(&path)
        .status();

    if matches!(status, Ok(status) if status.success()) {
        return Ok(());
    }

    launch_with_xdg_open(&path, "editar")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn card_design_registry_has_unique_ids() {
        let ids: Vec<_> = card_designs().map(|design| design.id()).collect();
        let unique: HashSet<_> = ids.iter().copied().collect();

        assert_eq!(ids.len(), SHOWCASE_CARDS);
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn unknown_card_design_falls_back_to_basic() {
        assert_eq!(card_design("does-not-exist").id(), "basic");
    }

    #[test]
    fn long_paths_are_shortened_without_losing_both_ends() {
        let shortened = middle_ellipsis("/home/pedro/Descargas/biblioteca/Terradraga.epub", 21);

        assert_eq!(shortened.chars().count(), 21);
        assert!(shortened.starts_with("/home/pedr"));
        assert!(shortened.ends_with("draga.epub"));
    }

    #[test]
    fn file_sizes_are_human_readable() {
        assert_eq!(human_file_size(500), "500 B");
        assert_eq!(human_file_size(2 * 1024), "2.0 KB");
        assert_eq!(human_file_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn cover_cache_uses_the_persisted_epub_hash() {
        let hash = "A".repeat(64);

        assert_eq!(
            stable_cover_cache_key(Some(&hash), 42, "/books/example.epub"),
            format!("{COVER_CACHE_VERSION}-{}", "a".repeat(64))
        );
        assert_eq!(
            stable_cover_cache_key_with_version("v1", Some(&hash), 42, "/books/example.epub"),
            format!("v1-{}", "a".repeat(64))
        );
    }

    #[test]
    fn invalid_cover_hashes_cannot_escape_the_cache_directory() {
        let key = stable_cover_cache_key(Some("../../outside"), 42, "/books/example.epub");

        assert!(key.starts_with(&format!("{COVER_CACHE_VERSION}-fallback-")));
        assert!(!key.contains('/'));
    }

    #[test]
    fn large_covers_are_cached_as_bounded_thumbnails() {
        let original = image::DynamicImage::new_rgb8(1200, 1800);
        let mut encoded = std::io::Cursor::new(Vec::new());
        original
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();

        let thumbnail = thumbnail_cover_bytes(&encoded.into_inner()).unwrap();
        let decoded = image::load_from_memory(&thumbnail).unwrap();

        assert_eq!(decoded.width(), COVER_THUMBNAIL_WIDTH);
        assert_eq!(decoded.height(), COVER_THUMBNAIL_HEIGHT);
    }
}
