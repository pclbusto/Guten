use cosmic::Element;
use cosmic::iced::widget::text_editor::Catalog as TextEditorCatalog;
use cosmic::iced::{Alignment, Color, Length};
use cosmic::theme::iced::TextEditor as TextEditorClass;
use cosmic::widget::button::ButtonClass;
use cosmic::widget::{button, column, container, icon, row, text, text_editor, tooltip};

use crate::app::{Message, OpenTab};
use crate::document::ProjectModel;
use crate::ui::syntax_highlighter::{self, Language};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[allow(clippy::too_many_arguments)]
pub fn view_editor<'a>(
    project_loaded: bool,
    active_tab: Option<&'a OpenTab>,
    active_tab_id: Option<&str>,
    preview_panel_open: bool,
    tabs: &'a [OpenTab],
    recent_projects: &'a [PathBuf],
) -> Element<'a, Message> {
    if !project_loaded {
        return view_welcome(recent_projects);
    }

    let resource_tabs: Element<'a, Message> = if tabs.is_empty() {
        cosmic::widget::Space::new().width(Length::Shrink).into()
    } else {
        let tab_buttons: Vec<Element<'a, Message>> = tabs
            .iter()
            .map(|tab| {
                let is_active = active_tab_id == Some(tab.resource_id.as_str());
                let dirty_mark = if tab.dirty { " ●" } else { "" };
                let label = format!("{}{}", tab.resource_id, dirty_mark);
                let tab_button = button::custom(text::body(label))
                    .class(if is_active {
                        ButtonClass::Standard
                    } else {
                        ButtonClass::Text
                    })
                    .on_press(Message::SelectResource(tab.resource_id.clone()))
                    .padding([6, 10]);
                let close_button = button::icon(icon::from_name("window-close-symbolic"))
                    .on_press(Message::CloseTab(tab.resource_id.clone()))
                    .padding(4);
                row!(tab_button, close_button).spacing(0).into()
            })
            .collect();
        row(tab_buttons).spacing(4).into()
    };

    let preview_toggle: Element<'a, Message> = tooltip(
        button::icon(icon::from_name(if preview_panel_open {
            "view-conceal-symbolic"
        } else {
            "view-reveal-symbolic"
        }))
        .on_press(Message::TogglePreviewPanel)
        .padding(8),
        text::body(if preview_panel_open {
            "Ocultar vista previa"
        } else {
            "Mostrar vista previa"
        }),
        tooltip::Position::Bottom,
    )
    .into();

    let header = container(
        row!(
            resource_tabs,
            cosmic::widget::Space::new().width(Length::Fill),
            preview_toggle
        )
        .align_y(Alignment::Center)
        .spacing(8)
        .padding([8, 12]),
    )
    .width(Length::Fill);

    let body: Element<'a, Message> = if let Some(tab) = active_tab {
        let is_editable = ProjectModel::is_text_editable(&tab.media_type);
        let language = editor_language(&tab.media_type);
        let editor_widget: Element<'a, Message> = if is_editable {
            let editor = text_editor(&tab.content)
                .on_action(Message::EditorAction)
                .class(TextEditorClass::Custom(Box::new(editor_style)));
            if let Some(lang) = language {
                editor
                    .highlight_with::<crate::ui::syntax_highlighter::SyntaxHighlighter>(
                        lang,
                        syntax_highlighter::token_format,
                    )
                    .into()
            } else {
                editor.into()
            }
        } else {
            text_editor(&tab.content)
                .class(TextEditorClass::Custom(Box::new(editor_style)))
                .into()
        };
        container(editor_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(text::body("Seleccioná un recurso del panel lateral"))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    };

    column!(header, body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn editor_language(media_type: &str) -> Option<Language> {
    match media_type {
        "text/css" => Some(Language::Css),
        "application/xhtml+xml" | "text/html" => Some(Language::Html),
        _ => None,
    }
}

fn editor_style(theme: &cosmic::Theme, status: text_editor::Status) -> text_editor::Style {
    let mut style = TextEditorCatalog::style(theme, &TextEditorClass::Default, status);
    style.border = style.border.width(0.0);
    style
}

#[allow(clippy::too_many_arguments)]
pub fn view_preview<'a>(
    text: String,
    project: Option<&'a ProjectModel>,
    selected_id: Option<&str>,
    media_type: Option<&str>,
    style_map: &'a folio::content::StyleMap,
    css_rules: &'a [folio::css::CssRule],
    font_name_map: &'a folio::fonts::FontNameMap,
    scroll_y: f32,
    reader_metrics: Rc<RefCell<folio::reader::text_canvas::ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<folio::image_resources::ImageMetadataCache>>,
    reader_layout_cache: Rc<RefCell<folio::reader::text_canvas::ReaderLayoutCache>>,
    bg_color: Color,
    text_color: Color,
) -> Element<'a, Message> {
    if !matches!(media_type, Some("application/xhtml+xml" | "text/html")) {
        return container(text::body(
            "La vista previa solo está disponible para documentos HTML/XHTML.",
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into();
    }

    let mut blocks = folio::content::parse_xhtml(&text);
    // Resolve image sources to absolute paths so the canvas can load them.
    if let Some(project) = project
        && let Some(chapter_id) = selected_id
        && let Ok(chapter_path) = project.core.get_resource_path(chapter_id)
    {
        let epub_root = normalize_path(&project.core.workdir);
        for block in &mut blocks {
            match block {
                folio::content::ContentBlock::Image { src, .. } => {
                    if !src.starts_with('/')
                        && !src.contains("://")
                        && let Some(resolved) =
                            resolve_resource_path(&chapter_path, &epub_root, src)
                    {
                        *src = resolved.to_string_lossy().into_owned();
                    }
                }
                folio::content::ContentBlock::Inline { nodes, .. } => {
                    for node in nodes {
                        if let folio::content::InlineNode::Image { src, .. } = node
                            && !src.starts_with('/')
                            && !src.contains("://")
                            && let Some(resolved) =
                                resolve_resource_path(&chapter_path, &epub_root, src)
                        {
                            *src = resolved.to_string_lossy().into_owned();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let base_family = cosmic::iced::font::Family::Serif;

    let canvas = folio::reader::text_canvas::TextCanvas::new(
        &blocks,
        style_map,
        css_rules,
        style_map.p.font_size,
        bg_color,
        text_color,
        base_family,
        font_name_map,
        scroll_y,
        reader_metrics,
        image_metadata_cache,
        reader_layout_cache,
        |y| Message::SetReaderScroll(y),
        |delta, _smooth| Message::ReaderWheel(delta),
        |src| Message::ImageClicked(src),
    );

    cosmic::widget::canvas(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_welcome<'a>(recent_projects: &'a [PathBuf]) -> Element<'a, Message> {
    let mut content: Vec<Element<'a, Message>> = vec![
        text::heading("Rúbrica").size(32.0).into(),
        text::body("Editor de EPUB con libcosmic").size(16.0).into(),
        button::standard("Abrir proyecto")
            .on_press(Message::OpenProject)
            .into(),
        button::standard("Nuevo proyecto")
            .on_press(Message::NewProject)
            .into(),
    ];

    if !recent_projects.is_empty() {
        content.push(text::body("Proyectos recientes").size(14.0).into());
        for (idx, path) in recent_projects.iter().enumerate() {
            let label = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            content.push(
                button::standard(label)
                    .on_press(Message::OpenRecent(idx))
                    .width(Length::Fixed(320.0))
                    .into(),
            );
        }
    }

    container(
        column(content)
            .spacing(12)
            .align_x(Alignment::Center)
            .padding(40),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn resolve_resource_path(chapter_path: &Path, epub_root: &Path, src: &str) -> Option<PathBuf> {
    let src = src.trim();
    if src.is_empty() || src.starts_with('#') || src.starts_with("data:") || src.contains("://") {
        return None;
    }
    let resource = src.split(['?', '#']).next().unwrap_or_default();
    let relative = Path::new(resource.trim_start_matches('/'));
    let base = if resource.starts_with('/') {
        epub_root
    } else {
        chapter_path.parent()?
    };
    let resolved = normalize_path(&base.join(relative));
    resolved.starts_with(epub_root).then_some(resolved)
}
