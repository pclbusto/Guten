use cosmic::Element;
use cosmic::iced::font::{Style, Weight};
use cosmic::iced::widget::{rich_text, span};
use cosmic::iced::{Color, Font, Length};

use crate::content::{self, ContentBlock, StyleMap, StyledSpan};
use crate::css::{self, CssRule, ResolvedStyle};

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}

pub fn extract_heading(blocks: &[ContentBlock]) -> String {
    for block in blocks {
        if let ContentBlock::Heading { spans, .. } = block {
            let text: String = spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                return text;
            }
        }
    }
    String::new()
}

pub fn to_iced_span(
    ss: &StyledSpan,
    block_style: &content::BlockStyle,
    base_color: Color,
    font_family: cosmic::iced::font::Family,
) -> cosmic::iced::widget::text::Span<'static, String, Font> {
    let color = ss.color.unwrap_or(base_color);
    let mut s = span(ss.text.clone()).color(color);

    let bold = ss.bold || block_style.bold;
    let italic = ss.italic || block_style.italic;

    s = s.font(Font {
        family: font_family,
        weight: if bold { Weight::Bold } else { Weight::Normal },
        style: if italic { Style::Italic } else { Style::Normal },
        ..Default::default()
    });

    if ss.underline || block_style.underline {
        s = s.underline(true);
    }
    if ss.strikethrough {
        s = s.strikethrough(true);
    }
    if let Some(size) = ss.size {
        s = s.size(size);
    }
    if let Some(ref link) = ss.link {
        s = s.link(link.clone());
    }

    s
}

fn merge_styles(base: &content::BlockStyle, css: &ResolvedStyle) -> content::BlockStyle {
    content::BlockStyle {
        font_size: css.font_size.unwrap_or(base.font_size),
        bold: css.bold || base.bold,
        italic: css.italic || base.italic,
        underline: css.underline || base.underline,
        color: css.color.or(base.color),
        align: css.align.or(base.align),
        line_height: base.line_height,
    }
}

pub fn render_blocks_to_elements<M: Clone + 'static>(
    blocks: &[ContentBlock],
    style_map: &StyleMap,
    text_color: Color,
    font_family: cosmic::iced::font::Family,
    css_rules: &[CssRule],
    on_link_click: impl Fn(String) -> M + Clone + 'static,
) -> Vec<Element<'static, M>> {
    let mut elements: Vec<Element<'static, M>> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::Heading {
                level,
                spans,
                classes,
            } => {
                let bs = style_map.heading_style(*level);
                let tag = format!("h{}", level);
                let css_style = css::resolve_style(&tag, classes, css_rules);
                let block_style = merge_styles(bs, &css_style);
                let block_color = css_style.color.unwrap_or(text_color);
                let block_font = css_style
                    .font_family
                    .as_ref()
                    .map(|f| {
                        cosmic::iced::font::Family::Name(Box::leak(f.clone().into_boxed_str()))
                    })
                    .unwrap_or(font_family);

                let iced_spans: Vec<_> = spans
                    .iter()
                    .map(|s| to_iced_span(s, &block_style, block_color, block_font))
                    .collect();
                let mut rich = rich_text(iced_spans).size(block_style.font_size);
                if let Some(lh) = block_style.line_height {
                    rich = rich.line_height(lh);
                }
                if let Some(align) = block_style.align.or(css_style.align) {
                    rich = rich.align_x(align);
                }
                elements.push(rich.on_link_click(on_link_click.clone()).into());
                elements.push(
                    cosmic::widget::Space::new()
                        .width(Length::Fill)
                        .height(8.0)
                        .into(),
                );
            }
            ContentBlock::Paragraph { spans, classes } => {
                let css_style = css::resolve_style("p", classes, css_rules);
                let block_style = merge_styles(&style_map.p, &css_style);
                let block_color = css_style.color.unwrap_or(text_color);
                let block_font = css_style
                    .font_family
                    .as_ref()
                    .map(|f| {
                        cosmic::iced::font::Family::Name(Box::leak(f.clone().into_boxed_str()))
                    })
                    .unwrap_or(font_family);

                let iced_spans: Vec<_> = spans
                    .iter()
                    .map(|s| to_iced_span(s, &block_style, block_color, block_font))
                    .collect();
                let mut rich = rich_text(iced_spans).size(block_style.font_size);
                if let Some(align) = block_style.align.or(css_style.align) {
                    rich = rich.align_x(align);
                }
                elements.push(rich.on_link_click(on_link_click.clone()).into());
                elements.push(
                    cosmic::widget::Space::new()
                        .width(Length::Fill)
                        .height(4.0)
                        .into(),
                );
            }
            ContentBlock::Image { src, alt } => {
                let label = if alt.is_empty() {
                    "[Imagen]".into()
                } else {
                    format!("[Imagen: {}]", alt)
                };
                let img_span = span(label)
                    .color(style_map.image_color)
                    .underline(true)
                    .link(src.clone());
                let rich = rich_text([img_span])
                    .size(12.0)
                    .on_link_click(on_link_click.clone());
                elements.push(rich.into());
                elements.push(
                    cosmic::widget::Space::new()
                        .width(Length::Fill)
                        .height(6.0)
                        .into(),
                );
            }
            ContentBlock::Separator => {
                elements.push(
                    cosmic::widget::Space::new()
                        .width(Length::Fill)
                        .height(12.0)
                        .into(),
                );
            }
        }
    }

    elements
}
