use cosmic::iced::{Color, Length};
use cosmic::widget::{self, button, column, container, row, scrollable, text};
use cosmic::Element;
use cosmic::widget::Row;

use crate::viewer::ViewerMessage;
use crate::{AspectRatio, Orientation, ViewerState};

pub fn sidebar_view<'a>(state: &'a ViewerState) -> Element<'a, ViewerMessage> {
    let content = column!(
        aspect_ratio_section(state),
        divider(),
        rotate_section(),
        divider(),
        flip_section(),
    )
    .spacing(16)
    .padding(16);

    let scroll = scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill);

    let sidebar_bg = state.theme.sidebar_bg();
    let sidebar_text = state.theme.text_color();

    container(scroll)
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .style(move |_theme: &cosmic::Theme| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(sidebar_bg)),
            text_color: Some(sidebar_text),
            ..Default::default()
        })
        .into()
}

fn divider<'a>() -> Element<'a, ViewerMessage> {
    container(widget::Space::new().width(Length::Fill).height(1.0))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_theme: &cosmic::Theme| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(Color::from_rgba8(60, 60, 68, 1.0))),
            ..Default::default()
        })
        .into()
}

fn aspect_ratio_section<'a>(state: &'a ViewerState) -> Element<'a, ViewerMessage> {
    let label = text::body("Aspect Ratio").size(13.0);

    let ratios = [
        (AspectRatio::Free, "Free"),
        (AspectRatio::Original, "Original"),
        (AspectRatio::Square, "Square"),
        (AspectRatio::Ratio5x4, "5:4"),
        (AspectRatio::Ratio4x3, "4:3"),
        (AspectRatio::Ratio3x2, "3:2"),
        (AspectRatio::Ratio16x9, "16:9"),
    ];

    let mut ratio_buttons = column!().spacing(6);

    for chunk in ratios.chunks(2) {
        let mut row_widgets: Vec<Element<'_, ViewerMessage>> = Vec::new();
        for (ratio, name) in chunk {
            let btn = if state.aspect_ratio == *ratio {
                button::suggested(*name)
                    .on_press(ViewerMessage::SetAspectRatio(*ratio))
                    .width(Length::Fill)
            } else {
                button::standard(*name)
                    .on_press(ViewerMessage::SetAspectRatio(*ratio))
                    .width(Length::Fill)
            };
            row_widgets.push(btn.into());
        }
        let r = Row::with_children(row_widgets).spacing(6);
        ratio_buttons = ratio_buttons.push(r);
    }

    let orientation = row!(
        button::standard("\u{2194}")
            .on_press(ViewerMessage::SetOrientation(Orientation::Landscape))
            .width(Length::Fill),
        button::standard("\u{2195}")
            .on_press(ViewerMessage::SetOrientation(Orientation::Portrait))
            .width(Length::Fill),
    )
    .spacing(6);

    column!(label, ratio_buttons, orientation).spacing(10).into()
}

fn rotate_section<'a>() -> Element<'a, ViewerMessage> {
    let label = text::body("Rotate").size(13.0);

    let buttons = row!(
        button::standard("\u{21ba}")
            .on_press(ViewerMessage::RotateCCW)
            .width(Length::Fill),
        button::standard("\u{21bb}")
            .on_press(ViewerMessage::RotateCW)
            .width(Length::Fill),
    )
    .spacing(6);

    column!(label, buttons).spacing(10).into()
}

fn flip_section<'a>() -> Element<'a, ViewerMessage> {
    let label = text::body("Flip").size(13.0);

    let buttons = row!(
        button::standard("\u{2194}")
            .on_press(ViewerMessage::FlipH)
            .width(Length::Fill),
        button::standard("\u{2195}")
            .on_press(ViewerMessage::FlipV)
            .width(Length::Fill),
    )
    .spacing(6);

    column!(label, buttons).spacing(10).into()
}
