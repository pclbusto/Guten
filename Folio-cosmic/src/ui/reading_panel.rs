use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, button, dropdown, row, settings, slider};

use crate::app::Message;
use crate::settings::{DEFAULT_READER_FONT_SIZE_PT, ReaderSettings};
use folio::fonts::EpubFont;
use std::collections::HashSet;

pub fn font_options(epub_fonts: &[EpubFont]) -> Vec<String> {
    let mut options = vec!["Sans".to_string(), "Serif".to_string()];
    let mut listed = HashSet::from(["Sans", "Serif"]);
    for font in epub_fonts {
        if listed.insert(font.family_name.as_str()) {
            options.push(font.family_name.clone());
        }
    }
    options
}

pub fn view_reading_panel<'a>(
    reader_settings: &'a ReaderSettings,
    epub_fonts: &'a [EpubFont],
) -> Element<'a, Message> {
    let themes = vec!["Día".to_string(), "Sepia".to_string(), "Noche".to_string()];
    let selected_theme = match reader_settings.current_profile.as_str() {
        "day" => Some(0),
        "sepia" => Some(1),
        "night" => Some(2),
        _ => None,
    };
    let fonts = font_options(epub_fonts);
    let selected_font = fonts
        .iter()
        .position(|font| font == &reader_settings.font_family)
        .or(Some(0));
    let font_percent = reader_settings.font_size_pt / DEFAULT_READER_FONT_SIZE_PT * 100.0;

    let appearance = settings::section()
        .title("Apariencia")
        .add(settings::item(
            "Modo",
            dropdown(themes, selected_theme, Message::SelectProfile),
        ))
        .add(settings::item(
            "Fuente",
            dropdown(fonts, selected_font, Message::SelectFont),
        ));
    let typography = settings::section()
        .title("Texto")
        .add(settings::item(
            format!("Tamaño ({font_percent:.0}%)"),
            slider(60.0..=200.0, font_percent, Message::FontSizeChanged)
                .step(5.0)
                .width(Length::Fixed(180.0)),
        ))
        .add(settings::item(
            format!("Márgenes ({:.1})", reader_settings.margin_em),
            row!(
                button::standard("−").on_press(Message::MarginDecrease),
                button::standard("+").on_press(Message::MarginIncrease),
            )
            .spacing(4),
        ));

    widget::container(settings::view_column(vec![
        appearance.into(),
        typography.into(),
    ]))
    .width(Length::Fixed(380.0))
    .height(Length::Fill)
    .padding(12)
    .style(|theme: &cosmic::Theme| {
        let cosmic = theme.cosmic();
        widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic.background.component.base.into(),
            )),
            ..Default::default()
        }
    })
    .into()
}
