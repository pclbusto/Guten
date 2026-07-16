use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, button, column, container, scrollable, text, text_input};

use crate::app::Message;
use folio::content::{self, ContentBlock};

pub fn view_sidebar<'a>(
    search_query: &'a str,
    search_results: &'a [gutencore::SearchResult],
) -> Element<'a, Message> {
    let input = text_input("Buscar en el libro...", search_query)
        .on_input(Message::SearchQueryChanged)
        .on_submit(|_| Message::ExecuteSearch)
        .width(Length::Fill);
    let results: Vec<Element<'a, Message>> = search_results
        .iter()
        .enumerate()
        .map(|(idx, r)| {
            let clean = content::parse_xhtml(&r.snippet)
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Heading { spans, .. } | ContentBlock::Paragraph { spans, .. } => {
                        Some(
                            spans
                                .iter()
                                .map(|s| s.text.as_str())
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    }
                    ContentBlock::Inline { nodes, .. } => Some(
                        nodes
                            .iter()
                            .filter_map(|node| match node {
                                content::InlineNode::Text(span) => Some(span.text.as_str()),
                                content::InlineNode::LineBreak => Some(" "),
                                content::InlineNode::Image { .. } => None,
                            })
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let tr = if clean.len() > 80 {
                format!("{}...", &clean[..80.min(clean.len())])
            } else {
                clean
            };
            button::standard(tr)
                .on_press(Message::SelectSearchResult(idx))
                .width(Length::Fill)
                .into()
        })
        .collect();
    let count = text::body(format!("{} resultados", search_results.len())).size(12.0);
    let res_col = cosmic::widget::Column::with_children(results).spacing(1);
    let panel_content = column!(
        input,
        count,
        scrollable(res_col).width(Length::Fill).height(Length::Fill)
    )
    .spacing(4)
    .padding(4);

    container(panel_content.width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(300.0))
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
