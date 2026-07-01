use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, button, column, container, row, text};

use crate::app::Message;
use crate::settings::{DEFAULT_READER_FONT_SIZE_PT, ReaderSettings};
use folio::fonts::EpubFont;
use std::collections::HashSet;

pub fn view_reading_panel<'a>(
    settings: &'a ReaderSettings,
    epub_fonts: &'a [EpubFont],
) -> Element<'a, Message> {
    let profile = settings.current_profile();
    let theme_text = format!("Tema: {}", profile.name);
    let current_font = if settings.font_family.is_empty() || settings.font_family == "Sans" {
        "Sans".to_string()
    } else {
        settings.font_family.clone()
    };

    let mut font_buttons: Vec<Element<'a, Message>> = vec![
        button::standard("Sans")
            .on_press(Message::SelectFont("Sans".into()))
            .width(Length::Fill)
            .into(),
    ];
    let mut listed_families = HashSet::new();
    for epub_font in epub_fonts {
        let name = &epub_font.family_name;
        if !listed_families.insert(name.as_str()) {
            continue;
        }
        let is_active = &settings.font_family == name;
        let btn: Element<'a, Message> = if is_active {
            button::suggested(name.as_str())
                .on_press(Message::SelectFont(name.clone()))
                .width(Length::Fill)
                .into()
        } else {
            button::standard(name.as_str())
                .on_press(Message::SelectFont(name.clone()))
                .width(Length::Fill)
                .into()
        };
        font_buttons.push(btn);
    }
    let font_row = cosmic::widget::Column::with_children(font_buttons).spacing(4);

    container(
        column!(
            text::body("Lectura").size(14.0),
            text::body(theme_text).size(12.0),
            row!(
                button::standard("D\u{ed}a")
                    .on_press(Message::SetProfile("day".into()))
                    .width(Length::Fill),
                button::standard("Sepia")
                    .on_press(Message::SetProfile("sepia".into()))
                    .width(Length::Fill),
                button::standard("Noche")
                    .on_press(Message::SetProfile("night".into()))
                    .width(Length::Fill)
            )
            .spacing(4),
            widget::Space::new().height(8.0).width(Length::Fill),
            text::body(format!("Fuente: {}", current_font)).size(12.0),
            font_row,
            widget::Space::new().height(8.0).width(Length::Fill),
            text::body(format!(
                "Tama\u{f1}o: {:.0}%",
                settings.font_size_pt / DEFAULT_READER_FONT_SIZE_PT * 100.0
            ))
            .size(12.0),
            row!(
                button::standard("\u{2212}")
                    .on_press(Message::FontDecrease)
                    .width(Length::Fill),
                button::standard("Aa")
                    .on_press(Message::FontReset)
                    .width(Length::Fill),
                button::standard("+")
                    .on_press(Message::FontIncrease)
                    .width(Length::Fill)
            )
            .spacing(4),
            widget::Space::new().height(8.0).width(Length::Fill),
            text::body(format!("Interlineado: {:.1}", settings.line_height)).size(12.0),
            row!(
                button::standard("\u{2212}")
                    .on_press(Message::LineHeightDecrease)
                    .width(Length::Fill),
                button::standard("+")
                    .on_press(Message::LineHeightIncrease)
                    .width(Length::Fill)
            )
            .spacing(4),
            widget::Space::new().height(8.0).width(Length::Fill),
            text::body(format!("M\u{e1}rgenes: {:.1}", settings.margin_em)).size(12.0),
            row!(
                button::standard("\u{2212}")
                    .on_press(Message::MarginDecrease)
                    .width(Length::Fill),
                button::standard("+")
                    .on_press(Message::MarginIncrease)
                    .width(Length::Fill)
            )
            .spacing(4),
        )
        .spacing(6)
        .padding(16),
    )
    .width(Length::Fixed(260.0))
    .height(Length::Fill)
    .style(|theme: &cosmic::Theme| {
        let c = theme.cosmic();
        widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                c.background.component.base.into(),
            )),
            ..Default::default()
        }
    })
    .into()
}
