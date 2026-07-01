use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, button, column, container, row, scrollable, text, text_input};

use crate::app::{Message, SidebarTab};
use folio::content::{self, ContentBlock};

use gutencore::TocEntry;

pub fn view_sidebar<'a>(
    sidebar_tab: &'a SidebarTab,
    toc_entries: &'a [TocEntry],
    search_query: &'a str,
    search_results: &'a [gutencore::SearchResult],
) -> Element<'a, Message> {
    let toc_tab = button::standard("Contenido")
        .on_press(Message::SelectSidebarTab(SidebarTab::Toc))
        .width(Length::Fill);
    let search_tab = button::standard("Buscar")
        .on_press(Message::SelectSidebarTab(SidebarTab::Search))
        .width(Length::Fill);
    let tabs = row!(toc_tab, search_tab).spacing(2).padding(4);

    let panel_content: Element<'a, Message> = match sidebar_tab {
        SidebarTab::Toc => {
            let items: Vec<Element<'a, Message>> = toc_entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| {
                    let indent = "  ".repeat(entry.level as usize);
                    button::standard(format!("{}{}", indent, entry.title))
                        .on_press(Message::SelectTocEntry(idx))
                        .width(Length::Fill)
                        .into()
                })
                .collect();
            let col = cosmic::widget::Column::with_children(items)
                .spacing(1)
                .padding(4);
            scrollable(col)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        SidebarTab::Search => {
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
                            ContentBlock::Heading { spans, .. }
                            | ContentBlock::Paragraph { spans, .. } => Some(
                                spans
                                    .iter()
                                    .map(|s| s.text.as_str())
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
            column!(
                input,
                count,
                scrollable(res_col).width(Length::Fill).height(Length::Fill)
            )
            .spacing(4)
            .padding(4)
            .into()
        }
    };

    container(
        column!(tabs, panel_content)
            .width(Length::Fill)
            .height(Length::Fill),
    )
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
