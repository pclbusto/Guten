use cosmic::Element;
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, container, text};

use crate::app::Message;

pub fn view_footer(
    spine_progress: (usize, usize),
    chapter_percent: f32,
    book_percent: f32,
    bg: Color,
    fg: Color,
) -> Element<'static, Message> {
    let (cur, total) = spine_progress;
    let label = if total > 0 {
        format!(
            "Cap\u{ed}tulo {cur} de {total} \u{b7} {chapter_percent:.0}% cap\u{ed}tulo \u{b7} {book_percent:.0}% libro"
        )
    } else {
        String::new()
    };

    container(text::body(label).size(12.0))
        .width(Length::Fill)
        .padding([4, 16])
        .align_x(Alignment::Center)
        .style(move |_theme: &cosmic::Theme| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(bg)),
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}
