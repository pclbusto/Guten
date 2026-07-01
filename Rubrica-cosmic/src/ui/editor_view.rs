use cosmic::Element;
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{button, column, container, row, text, text_editor};

use crate::app::{EditorTab, Message};
use crate::document::ProjectModel;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub fn view_editor<'a>(
    project_loaded: bool,
    project: Option<&'a ProjectModel>,
    selected_id: Option<&'a str>,
    selected_media_type: Option<&'a str>,
    content: &'a text_editor::Content,
    dirty: bool,
    active_tab: EditorTab,
    recent_projects: &'a [PathBuf],
    reader_scroll_y: f32,
    reader_metrics: Rc<RefCell<folio::reader::text_canvas::ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<folio::image_resources::ImageMetadataCache>>,
    reader_layout_cache: Rc<RefCell<folio::reader::text_canvas::ReaderLayoutCache>>,
    style_map: &'a folio::content::StyleMap,
    css_rules: &'a [folio::css::CssRule],
    font_name_map: &'a folio::fonts::FontNameMap,
    bg_color: Color,
    text_color: Color,
) -> Element<'a, Message> {
    if !project_loaded {
        return view_welcome(recent_projects);
    }

    let header_text = if let Some(id) = selected_id {
        let dirty_mark = if dirty { " ●" } else { "" };
        format!("{}{}", id, dirty_mark)
    } else {
        "Seleccioná un recurso del panel lateral".into()
    };

    let editor_tab = if active_tab == EditorTab::Editor {
        button::suggested("Editor")
    } else {
        button::standard("Editor")
    }
    .on_press(Message::SelectEditorTab(EditorTab::Editor));

    let preview_tab = if active_tab == EditorTab::Preview {
        button::suggested("Vista Previa")
    } else {
        button::standard("Vista Previa")
    }
    .on_press(Message::SelectEditorTab(EditorTab::Preview));

    let tabs = row!(editor_tab, preview_tab).spacing(4);

    let header = container(
        row!(
            text::body(header_text).size(14.0),
            cosmic::widget::Space::new().width(Length::Fill),
            tabs,
            button::standard("Guardar").on_press(Message::SaveCurrent),
        )
        .align_y(Alignment::Center)
        .spacing(8)
        .padding([8, 12]),
    )
    .width(Length::Fill);

    let is_editable = selected_media_type
        .map(ProjectModel::is_text_editable)
        .unwrap_or(false);

    let body: Element<'a, Message> = match active_tab {
        EditorTab::Editor => {
            let editor_widget = if is_editable {
                text_editor(content).on_action(Message::EditorAction)
            } else {
                text_editor(content)
            };
            container(editor_widget)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        EditorTab::Preview => view_preview(
            content.text(),
            project,
            selected_id,
            selected_media_type,
            style_map,
            css_rules,
            font_name_map,
            reader_scroll_y,
            reader_metrics,
            image_metadata_cache,
            reader_layout_cache,
            bg_color,
            text_color,
        ),
    };

    column!(header, body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[allow(clippy::too_many_arguments)]
fn view_preview<'a>(
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
    if !matches!(
        media_type,
        Some("application/xhtml+xml" | "text/html")
    ) {
        return container(
            text::body("La vista previa solo está disponible para documentos HTML/XHTML."),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into();
    }

    let mut blocks = folio::content::parse_xhtml(&text);
    // Resolve image sources to absolute paths so the canvas can load them.
    if let Some(project) = project {
        if let Some(chapter_id) = selected_id {
            if let Ok(chapter_path) = project.core.get_resource_path(chapter_id) {
                let epub_root = normalize_path(&project.core.workdir);
                for block in &mut blocks {
                    if let folio::content::ContentBlock::Image { src, .. } = block {
                        if !src.starts_with('/') && !src.contains("://") {
                            if let Some(resolved) =
                                resolve_resource_path(&chapter_path, &epub_root, src)
                            {
                                *src = resolved.to_string_lossy().into_owned();
                            }
                        }
                    }
                }
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
