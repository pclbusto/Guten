use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{self, button, column, container, row, scrollable, text, text_input};

use crate::app::{Message, SearchResult, SidebarTab};
use crate::document::ProjectModel;

pub fn view_sidebar<'a>(
    sidebar_tab: &'a SidebarTab,
    project: &'a ProjectModel,
    selected_id: &'a Option<String>,
    search_query: &'a str,
    search_results: &'a [SearchResult],
    width: f32,
) -> Element<'a, Message> {
    let resources_tab = button::standard("Recursos")
        .on_press(Message::SelectSidebarTab(SidebarTab::Resources))
        .width(Length::Fill);
    let search_tab = button::standard("Buscar")
        .on_press(Message::SelectSidebarTab(SidebarTab::Search))
        .width(Length::Fill);
    let tabs = row!(resources_tab, search_tab).spacing(2).padding(4);

    let panel_content: Element<'a, Message> = match sidebar_tab {
        SidebarTab::Resources => view_resources(project, selected_id),
        SidebarTab::Search => view_search(search_query, search_results),
    };

    container(
        column!(tabs, panel_content)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(width))
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

fn view_resources<'a>(
    project: &'a ProjectModel,
    selected_id: &'a Option<String>,
) -> Element<'a, Message> {
    let groups = project.grouped_resources();
    let mut children: Vec<Element<'a, Message>> = Vec::new();

    for group in groups {
        children.push(
            text::body(group.label)
                .size(12.0)
                .width(Length::Fill)
                .into(),
        );
        for item in group.items {
            let is_selected = selected_id.as_ref() == Some(&item.id);
            let label = format!("{} ({})", item.id, item.media_type);
            let btn = if is_selected {
                button::suggested(label)
            } else {
                button::standard(label)
            }
            .on_press(Message::SelectResource(item.id))
            .width(Length::Fill);
            children.push(btn.into());
        }
        children.push(cosmic::widget::Space::new().height(8).into());
    }

    let col = cosmic::widget::Column::with_children(children)
        .spacing(1)
        .padding(4);
    scrollable(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_search<'a>(
    search_query: &'a str,
    search_results: &'a [SearchResult],
) -> Element<'a, Message> {
    let input = text_input("Buscar en el proyecto...", search_query)
        .on_input(Message::SearchQueryChanged)
        .on_submit(|_| Message::ExecuteSearch)
        .width(Length::Fill);

    let results: Vec<Element<'a, Message>> = search_results
        .iter()
        .enumerate()
        .map(|(idx, r)| {
            let snippet = if r.snippet.len() > 80 {
                format!("{}...", &r.snippet[..80.min(r.snippet.len())])
            } else {
                r.snippet.clone()
            };
            let label = format!("{}: {}", r.chapter_id, snippet);
            button::standard(label)
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
