use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, icon, row, text, text_input};

use crate::app::{Message, RenameDialog};

pub fn view_rename_dialog<'a>(dialog: &'a RenameDialog) -> Element<'a, Message> {
    let header = row!(
        text::heading("Renombrar recurso").size(18.0),
        cosmic::widget::Space::new().width(Length::Fill),
        button::icon(icon::from_name("window-close-symbolic"))
            .on_press(Message::CancelRename)
            .padding(8)
    )
    .align_y(Alignment::Center)
    .spacing(12);

    let current_label = text::body(format!("Ruta actual: {}", dialog.current_href)).size(13.0);
    let input = text_input("Nueva ruta relativa...", &dialog.new_href)
        .on_input(Message::RenameInputChanged)
        .on_submit(|_| Message::ConfirmRename)
        .width(Length::Fill);

    let actions = row!(
        cosmic::widget::Space::new().width(Length::Fill),
        button::standard("Cancelar").on_press(Message::CancelRename),
        button::suggested("Renombrar").on_press(Message::ConfirmRename)
    )
    .align_y(Alignment::Center)
    .spacing(8);

    let content = column!(
        header,
        column!(current_label, input).spacing(8),
        actions
    )
    .spacing(16)
    .width(Length::Fill);

    container(
        container(content)
            .width(Length::Fixed(480.0))
            .padding(20)
            .style(|theme: &cosmic::Theme| {
                let c = theme.cosmic();
                cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        c.background.base.into(),
                    )),
                    border: cosmic::iced::Border {
                        radius: 12.0.into(),
                        width: 1.0,
                        color: c.background.divider.into(),
                    },
                    ..Default::default()
                }
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(|theme: &cosmic::Theme| {
        let c = theme.cosmic();
        cosmic::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                c.background.base.into(),
            )),
            ..Default::default()
        }
    })
    .into()
}
