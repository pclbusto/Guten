use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, checkbox, column, container, icon, row, scrollable, text};

use crate::app::Message;

pub fn view_toc_dialog<'a>(data: &'a [gutencore::DocToc]) -> Element<'a, Message> {
    let mut doc_items: Vec<Element<'a, Message>> = Vec::new();

    for (doc_idx, doc) in data.iter().enumerate() {
        let doc_label = format!("{} ({})", doc.title, doc.href);
        let doc_checkbox = checkbox(doc.include)
            .label(doc_label)
            .on_toggle(move |checked| Message::TocDialogDocInclude(doc_idx, checked));

        let mut heading_items: Vec<Element<'a, Message>> = Vec::new();
        for (heading_idx, heading) in doc.items.iter().enumerate() {
            let indent = "  ".repeat((heading.level.saturating_sub(1)) as usize);
            let label = format!("{}{}", indent, heading.title);
            let h_checkbox = checkbox(heading.include)
                .label(label)
                .on_toggle(move |checked| {
                    Message::TocDialogHeadingInclude(doc_idx, heading_idx, checked)
                });
            heading_items.push(h_checkbox.into());
        }

        let doc_column = column!(
            doc_checkbox,
            column(heading_items).spacing(2).padding([0, 0, 0, 24])
        )
        .spacing(4);

        doc_items.push(doc_column.into());
    }

    let header = row!(
        text::heading("Tabla de contenidos").size(18.0),
        cosmic::widget::Space::new().width(Length::Fill),
        button::icon(icon::from_name("window-close-symbolic"))
            .on_press(Message::CloseTocDialog)
            .padding(8)
    )
    .align_y(Alignment::Center)
    .spacing(12);

    let selection_actions = row!(
        button::text("Todos").on_press(Message::TocDialogSelectAll(true)),
        button::text("Ninguno").on_press(Message::TocDialogSelectAll(false))
    )
    .align_y(Alignment::Center)
    .spacing(4);

    let confirm_actions = row!(
        button::standard("Cancelar").on_press(Message::CloseTocDialog),
        button::suggested("Generar TOC").on_press(Message::GenerateToc)
    )
    .align_y(Alignment::Center)
    .spacing(8);

    let actions = row!(
        selection_actions,
        cosmic::widget::Space::new().width(Length::Fill),
        confirm_actions
    )
    .align_y(Alignment::Center)
    .spacing(8);

    let content = column!(
        header,
        scrollable(column(doc_items).spacing(8))
            .width(Length::Fill)
            .height(Length::Fill),
        actions
    )
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill);

    container(
        container(content)
            .width(Length::Fixed(640.0))
            .height(Length::Fixed(520.0))
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
