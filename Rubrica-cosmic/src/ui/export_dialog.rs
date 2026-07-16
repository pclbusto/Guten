use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, icon, row, text};

use crate::app::{ExportFormat, Message};

pub fn view_export_dialog(selected: ExportFormat) -> Element<'static, Message> {
    let header = row!(
        text::heading("Exportar").size(18.0),
        cosmic::widget::Space::new().width(Length::Fill),
        button::icon(icon::from_name("window-close-symbolic"))
            .on_press(Message::CancelExport)
            .padding(8)
    )
    .align_y(Alignment::Center)
    .spacing(12);

    let epub = if selected == ExportFormat::Epub {
        button::suggested("EPUB (.epub)")
    } else {
        button::standard("EPUB (.epub)")
    }
    .on_press(Message::SelectExportFormat(ExportFormat::Epub));

    let text_file = if selected == ExportFormat::Text {
        button::suggested("Texto plano (.txt)")
    } else {
        button::standard("Texto plano (.txt)")
    }
    .on_press(Message::SelectExportFormat(ExportFormat::Text));

    let formats = column!(
        text::body("Formato de salida"),
        row!(epub, text_file).spacing(8)
    )
    .spacing(8);

    let actions = row!(
        cosmic::widget::Space::new().width(Length::Fill),
        button::standard("Cancelar").on_press(Message::CancelExport),
        button::suggested("Elegir destino").on_press(Message::ConfirmExport)
    )
    .align_y(Alignment::Center)
    .spacing(8);

    let content = column!(header, formats, actions)
        .spacing(20)
        .width(Length::Fill);

    container(
        container(content)
            .width(Length::Fixed(480.0))
            .padding(20)
            .style(|theme: &cosmic::Theme| {
                let c = theme.cosmic();
                cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(c.background.base.into())),
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
            background: Some(cosmic::iced::Background::Color(c.background.base.into())),
            ..Default::default()
        }
    })
    .into()
}
