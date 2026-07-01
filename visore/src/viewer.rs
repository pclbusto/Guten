use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, button, column, container, row, text};
use cosmic::Element;

use crate::sidebar;
use crate::{ViewerState, ViewerTheme};

#[derive(Debug, Clone)]
pub enum ViewerMessage {
    Cancel,
    Save,
    SaveAs(String),
    ToggleSaveDropdown,
    SetTheme(ViewerTheme),
    LoadImage(std::path::PathBuf),
    CropChanged(crate::CropRect),
    SetAspectRatio(crate::AspectRatio),
    SetOrientation(crate::Orientation),
    RotateCW,
    RotateCCW,
    FlipH,
    FlipV,
    CropApplied,
}

pub fn viewer_view<'a>(
    state: &'a ViewerState,
    show_save_dropdown: bool,
) -> Element<'a, ViewerMessage> {
    let header = viewer_header(state, show_save_dropdown);
    let body = viewer_body(state);

    column!(header, body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn viewer_header<'a>(
    state: &'a ViewerState,
    show_dropdown: bool,
) -> Element<'a, ViewerMessage> {
    let header_bg = state.theme.bg_color();
    let header_text = state.theme.text_color();

    let cancel_btn = button::standard("Cancel")
        .on_press(ViewerMessage::Cancel);

    let title = text::body(&state.title)
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    let is_dark = state.theme == ViewerTheme::Dark;
    let light_btn = if !is_dark {
        button::suggested("\u{2600}").on_press(ViewerMessage::SetTheme(ViewerTheme::Light))
    } else {
        button::standard("\u{2600}").on_press(ViewerMessage::SetTheme(ViewerTheme::Light))
    };
    let dark_btn = if is_dark {
        button::suggested("\u{263e}").on_press(ViewerMessage::SetTheme(ViewerTheme::Dark))
    } else {
        button::standard("\u{263e}").on_press(ViewerMessage::SetTheme(ViewerTheme::Dark))
    };

    let save_btn = button::suggested("Save \u{25bc}")
        .on_press(ViewerMessage::ToggleSaveDropdown);

    let save_area = column!(
        save_btn,
        if show_dropdown {
            column!(
                button::standard("Save as PNG")
                    .on_press(ViewerMessage::SaveAs("png".into()))
                    .width(Length::Fill),
                button::standard("Save as JPEG")
                    .on_press(ViewerMessage::SaveAs("jpeg".into()))
                    .width(Length::Fill),
                button::standard("Save as WebP")
                    .on_press(ViewerMessage::SaveAs("webp".into()))
                    .width(Length::Fill),
            )
            .padding(4)
            .spacing(2)
            .into()
        } else {
            let spacer: Element<'_, ViewerMessage> = widget::Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into();
            spacer
        }
    );

    container(
        row!(
            cancel_btn,
            widget::Space::new().width(Length::Fill),
            title,
            widget::Space::new().width(Length::Fill),
            light_btn,
            dark_btn,
            save_area,
        )
        .spacing(4)
        .padding([8, 12])
        .align_y(Alignment::Center),
    )
    .style(move |_theme: &cosmic::Theme| widget::container::Style {
        background: Some(cosmic::iced::Background::Color(header_bg)),
        text_color: Some(header_text),
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

fn viewer_body<'a>(state: &'a ViewerState) -> Element<'a, ViewerMessage> {
    let viewport = image_viewport(state);
    let sidebar_content = sidebar::sidebar_view(state);

    row!(viewport, sidebar_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn image_viewport<'a>(state: &'a ViewerState) -> Element<'a, ViewerMessage> {
        let bg = state.theme.bg_color();

    if let Some(ref handle) = state.image_handle {
        let image_widget = widget::image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(cosmic::iced::ContentFit::Contain);

        container(image_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &cosmic::Theme| widget::container::Style {
                background: Some(cosmic::iced::Background::Color(bg)),
                ..Default::default()
            })
            .into()
    } else {
        container(
            text::body("Arrastra una imagen o usa Abrir").size(16.0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_theme: &cosmic::Theme| widget::container::Style {
            background: Some(cosmic::iced::Background::Color(bg)),
            ..Default::default()
        })
        .into()
    }
}
