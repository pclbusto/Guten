use std::cell::RefCell;
use std::rc::Rc;

use cosmic::Element;
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, button, column, container, text};

use crate::app::Message;
use crate::settings::ReaderSettings;
use folio::content::{ContentBlock, StyleMap};
use folio::css::CssRule;
use folio::fonts::FontNameMap;
use folio::image_resources::ImageMetadataCache;
use folio::reader::renderer::parse_hex_color;
use folio::reader::text_canvas::{ReaderLayoutCache, ReaderMetrics, TextCanvas};

pub fn view_reader<'a>(
    document_loaded: bool,
    current_blocks: &'a [ContentBlock],
    style_map: &'a StyleMap,
    settings: &'a ReaderSettings,
    font_name_map: &'a FontNameMap,
    css_rules: &'a [CssRule],
    bg_color: Color,
    _scroll_id: cosmic::widget::Id,
    scroll_y: f32,
    reader_metrics: Rc<RefCell<ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<ImageMetadataCache>>,
    reader_layout_cache: Rc<RefCell<ReaderLayoutCache>>,
) -> Element<'a, Message> {
    if !document_loaded {
        let recent: Vec<Element<'a, Message>> = settings
            .recent_books
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Libro");
                button::standard(format!("Ctrl+{}   {}", index + 1, name))
                    .on_press(Message::OpenRecent(index))
                    .width(Length::Fixed(420.0))
                    .into()
            })
            .collect();
        let recent_list = cosmic::widget::Column::with_children(recent).spacing(4);
        return container(
            column!(
                text::heading("Folio").size(32.0),
                text::body("Abr\u{ed} un EPUB para comenzar").size(16.0),
                button::standard("Abrir libro").on_press(Message::OpenFile),
                text::heading("Libros recientes").size(20.0),
                recent_list,
            )
            .spacing(16)
            .align_x(Alignment::Center)
            .padding(40),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into();
    }

    let profile = settings.current_profile();
    let text_color = parse_hex_color(&profile.text_color).unwrap_or(Color::from_rgb8(26, 26, 26));

    let base_family = if settings.font_family.is_empty() || settings.font_family == "Sans" {
        cosmic::iced::font::Family::SansSerif
    } else if settings.font_family == "Serif" {
        cosmic::iced::font::Family::Serif
    } else {
        font_name_map
            .resolve(&settings.font_family, false, false)
            .map(|(family, _, _)| family)
            .unwrap_or(cosmic::iced::font::Family::SansSerif)
    };
    eprintln!(
        "[reader_view] base_family for '{}' -> {:?}",
        settings.font_family, base_family
    );

    let canvas = TextCanvas::new(
        current_blocks,
        style_map,
        css_rules,
        settings.font_size_pt as f32,
        bg_color,
        text_color,
        base_family,
        font_name_map,
        scroll_y,
        reader_metrics,
        image_metadata_cache,
        reader_layout_cache,
        |y| Message::SetReaderScroll(y),
        |delta, smooth| Message::ReaderWheel { delta, smooth },
        |src| Message::ImageClicked(src),
    );

    let canvas_widget = widget::canvas(canvas)
        .width(Length::Fill)
        .height(Length::Fill);

    let horizontal_margin = (settings.margin_em * settings.font_size_pt).clamp(0.0, 160.0) as u16;
    container(canvas_widget)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, horizontal_margin])
        .style(move |_theme: &cosmic::Theme| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(bg_color)),
            ..Default::default()
        })
        .into()
}
