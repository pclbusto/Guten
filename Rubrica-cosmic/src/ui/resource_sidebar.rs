use cosmic::Element;
use cosmic::iced::{Alignment, Color, Length, mouse};
use cosmic::widget::{
    button, column, container, icon, mouse_area, row, scrollable, segmented_button, tab_bar, text,
    text_input, tooltip,
};

use crate::app::{Message, SearchResult, SidebarTab};
use crate::document::ResourceGroup;
use gutencore::ResourceKind;

pub fn view_sidebar<'a>(
    sidebar_tab_model: &'a segmented_button::SingleSelectModel,
    groups: &'a [ResourceGroup],
    active_tab_id: Option<&'a str>,
    drag_id: Option<&'a str>,
    drop_id: Option<&'a str>,
    search_query: &'a str,
    search_results: &'a [SearchResult],
    width: f32,
) -> Element<'a, Message> {
    let tabs = tab_bar::horizontal(sidebar_tab_model)
        .on_activate(Message::SidebarTabSelected)
        .width(Length::Fill);

    let sidebar_tab = match sidebar_tab_model.text(sidebar_tab_model.active()) {
        Some("Buscar") => SidebarTab::Search,
        _ => SidebarTab::Resources,
    };

    let panel_content: Element<'a, Message> = match sidebar_tab {
        SidebarTab::Resources => view_resources(groups, active_tab_id, drag_id, drop_id),
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
        cosmic::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                c.background.component.base.into(),
            )),
            ..Default::default()
        }
    })
    .into()
}

fn view_resources<'a>(
    groups: &'a [ResourceGroup],
    active_tab_id: Option<&str>,
    drag_id: Option<&str>,
    drop_id: Option<&str>,
) -> Element<'a, Message> {
    let actions = row!(
        tooltip(
            button::icon(icon::from_name("list-add-symbolic"))
                .on_press(Message::CreateChapter)
                .padding(8),
            text::body("Nuevo capítulo"),
            tooltip::Position::Top,
        ),
        tooltip(
            button::icon(icon::from_name("format-text-code-symbolic"))
                .on_press(Message::CreateStyle)
                .padding(8),
            text::body("Nueva hoja de estilos"),
            tooltip::Position::Top,
        ),
        tooltip(
            button::icon(icon::from_name("image-x-generic-symbolic"))
                .on_press(Message::ImportImage)
                .padding(8),
            text::body("Importar imagen"),
            tooltip::Position::Top,
        ),
        tooltip(
            button::icon(icon::from_name("font-x-generic-symbolic"))
                .on_press(Message::ImportFont)
                .padding(8),
            text::body("Importar fuente"),
            tooltip::Position::Top,
        ),
        tooltip(
            button::icon(icon::from_name("view-list-symbolic"))
                .on_press(Message::OpenTocDialog)
                .padding(8),
            text::body("Tabla de contenidos"),
            tooltip::Position::Top,
        ),
    )
    .spacing(4)
    .padding([4, 8]);

    let mut items: Vec<Element<'a, Message>> = Vec::new();

    for group in groups {
        if group.items.is_empty() {
            continue;
        }

        items.push(
            container(text::body(group.label).size(11.0))
                .width(Length::Fill)
                .padding([8, 10, 4, 10])
                .style(|theme: &cosmic::Theme| {
                    let c = theme.cosmic();
                    cosmic::widget::container::Style {
                        text_color: Some(c.accent.base.into()),
                        ..Default::default()
                    }
                })
                .into(),
        );

        for item in &group.items {
            let is_active = active_tab_id == Some(item.id.as_str());
            let is_drag = drag_id == Some(item.id.as_str());
            let is_drop = drop_id == Some(item.id.as_str());
            let can_drag =
                group.kind == ResourceKind::Document && item.media_type == "application/xhtml+xml";
            let is_image = group.kind == ResourceKind::Image;

            let label = text::body(&item.id).size(13.0);

            let mut action_buttons: Vec<Element<'_, Message>> = Vec::new();
            action_buttons.push(
                tooltip(
                    button::icon(icon::from_name("document-edit-symbolic"))
                        .on_press(Message::StartRename(item.id.clone()))
                        .padding(2),
                    text::body("Renombrar"),
                    tooltip::Position::Top,
                )
                .into(),
            );
            if is_image {
                action_buttons.push(
                    tooltip(
                        button::icon(icon::from_name("emblem-favorite-symbolic"))
                            .on_press(Message::SetCover(item.id.clone()))
                            .padding(2),
                        text::body("Marcar como portada"),
                        tooltip::Position::Top,
                    )
                    .into(),
                );
            }
            let actions_row = row(action_buttons).spacing(2).padding([0, 6, 0, 0]);

            let content: Element<'_, Message> = container(
                row!(
                    container(label).width(Length::Fill).padding([6, 12]),
                    actions_row
                )
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .style(move |theme: &cosmic::Theme| {
                let c = theme.cosmic();
                let background = if is_drag {
                    c.accent.base
                } else if is_drop {
                    c.success.base
                } else if is_active {
                    c.background.component.hover
                } else {
                    c.background.component.base
                };
                let text = if is_drag || is_drop {
                    Color::WHITE
                } else {
                    c.background.on.into()
                };
                cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(background.into())),
                    text_color: Some(text),
                    ..Default::default()
                }
            })
            .into();

            let item_element: Element<'a, Message> = if can_drag {
                mouse_area(content)
                    .on_press(Message::SidebarResourceSelected(item.id.clone()))
                    .on_drag(Message::SidebarDragStart(item.id.clone()))
                    .on_enter(Message::SidebarDragOver(item.id.clone()))
                    .on_exit(Message::SidebarDragLeave(item.id.clone()))
                    .on_release(Message::SidebarDrop)
                    .interaction(mouse::Interaction::Grab)
                    .into()
            } else {
                mouse_area(content)
                    .on_press(Message::SidebarResourceSelected(item.id.clone()))
                    .into()
            };

            items.push(item_element);
        }
    }

    let list = mouse_area(
        scrollable(column(items).spacing(1))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_release(Message::SidebarDrop);

    column!(actions, list)
        .spacing(4)
        .padding([0, 0, 4, 0])
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
