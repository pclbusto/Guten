use crate::css::{self, CssRule};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::font::{Style, Weight};
use cosmic::iced::{Color, Font};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub link: Option<String>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum InlineNode {
    Text(StyledSpan),
    LineBreak,
    Image {
        src: String,
        alt: String,
        classes: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum InlineBlockKind {
    Heading(u8),
    Paragraph,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub text: String,
    pub header: bool,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub spans: Vec<StyledSpan>,
    pub classes: Vec<String>,
    pub depth: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BlockStyle {
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<Color>,
    pub align: Option<Horizontal>,
    pub line_height: Option<f32>,
}

impl BlockStyle {
    #[allow(dead_code)]
    pub fn to_font(&self) -> Font {
        Font {
            weight: if self.bold {
                Weight::Bold
            } else {
                Weight::Normal
            },
            style: if self.italic {
                Style::Italic
            } else {
                Style::Normal
            },
            ..Default::default()
        }
    }
}

impl Default for BlockStyle {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            bold: false,
            italic: false,
            underline: false,
            color: None,
            align: None,
            line_height: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<Color>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ContentBlock {
    Heading {
        level: u8,
        spans: Vec<StyledSpan>,
        classes: Vec<String>,
    },
    Paragraph {
        spans: Vec<StyledSpan>,
        classes: Vec<String>,
    },
    Image {
        src: String,
        alt: String,
    },
    Inline {
        kind: InlineBlockKind,
        nodes: Vec<InlineNode>,
        classes: Vec<String>,
    },
    Table {
        rows: Vec<TableRow>,
        classes: Vec<String>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
        classes: Vec<String>,
    },
    Separator,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StyleMap {
    pub h1: BlockStyle,
    pub h2: BlockStyle,
    pub h3: BlockStyle,
    pub h4: BlockStyle,
    pub h5: BlockStyle,
    pub h6: BlockStyle,
    pub p: BlockStyle,
    pub strong: InlineStyle,
    pub em: InlineStyle,
    pub image_color: Color,
}

impl Default for StyleMap {
    fn default() -> Self {
        Self {
            h1: BlockStyle {
                font_size: 26.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            h2: BlockStyle {
                font_size: 22.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            h3: BlockStyle {
                font_size: 18.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            h4: BlockStyle {
                font_size: 16.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            h5: BlockStyle {
                font_size: 14.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            h6: BlockStyle {
                font_size: 13.0,
                bold: true,
                line_height: Some(1.3),
                ..Default::default()
            },
            p: BlockStyle {
                font_size: 14.0,
                ..Default::default()
            },
            strong: InlineStyle {
                bold: true,
                italic: false,
                underline: false,
                color: None,
            },
            em: InlineStyle {
                bold: false,
                italic: true,
                underline: false,
                color: None,
            },
            image_color: Color::from_rgb8(0, 80, 180),
        }
    }
}

impl StyleMap {
    pub fn heading_style(&self, level: u8) -> &BlockStyle {
        match level {
            1 => &self.h1,
            2 => &self.h2,
            3 => &self.h3,
            4 => &self.h4,
            5 => &self.h5,
            _ => &self.h6,
        }
    }
}

fn decode_entities(text: &str) -> String {
    let decoded = decode_numeric_entities(text);
    decoded
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&iexcl;", "\u{00a1}")
        .replace("&iquest;", "\u{00bf}")
        .replace("&aacute;", "\u{00e1}")
        .replace("&eacute;", "\u{00e9}")
        .replace("&iacute;", "\u{00ed}")
        .replace("&oacute;", "\u{00f3}")
        .replace("&uacute;", "\u{00fa}")
        .replace("&ntilde;", "\u{00f1}")
        .replace("&Aacute;", "\u{00c1}")
        .replace("&Eacute;", "\u{00c9}")
        .replace("&Iacute;", "\u{00cd}")
        .replace("&Oacute;", "\u{00d3}")
        .replace("&Uacute;", "\u{00da}")
        .replace("&Ntilde;", "\u{00d1}")
        .replace("&uuml;", "\u{00fc}")
        .replace("&Uuml;", "\u{00dc}")
        .replace("&ordm;", "\u{00ba}")
        .replace("&ordf;", "\u{00aa}")
        .replace("&ldquo;", "\u{201c}")
        .replace("&rdquo;", "\u{201d}")
        .replace("&lsquo;", "\u{2018}")
        .replace("&rsquo;", "\u{2019}")
        .replace("&laquo;", "\u{00ab}")
        .replace("&raquo;", "\u{00bb}")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&bull;", "\u{2022}")
        .replace("&copy;", "\u{00a9}")
        .replace("&reg;", "\u{00ae}")
        .replace("&trade;", "\u{2122}")
        .replace("&euro;", "\u{20ac}")
}

fn decode_numeric_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut index = 0;

    while index < text.len() {
        let rest = &text[index..];
        if let Some(entity) = rest.strip_prefix("&#")
            && let Some(end) = entity.find(';')
        {
            let value = &entity[..end];
            let codepoint = value
                .strip_prefix(['x', 'X'])
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| value.parse::<u32>().ok());
            if let Some(character) = codepoint.and_then(char::from_u32) {
                if character == '\u{00a0}' {
                    decoded.push(' ');
                } else {
                    decoded.push(character);
                }
                index += 2 + end + 1;
                continue;
            }
        }

        let character = rest.chars().next().expect("non-empty text remainder");
        decoded.push(character);
        index += character.len_utf8();
    }

    decoded
}

fn parse_img_attributes(tail: &str) -> (String, String) {
    let mut src = String::new();
    let mut alt = String::new();
    let bytes = tail.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] == b'>' || bytes[i] == b'/' {
            break;
        }

        let name_start = i;
        while i < len
            && bytes[i] != b'='
            && bytes[i] != b' '
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        if name_start == i {
            i += 1;
            continue;
        }
        let attr_name = tail[name_start..i].to_lowercase();

        while i < len && (bytes[i] == b' ' || bytes[i] == b'=') {
            i += 1;
        }

        if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let val_start = i;
            while i < len && bytes[i] != quote {
                i += 1;
            }
            let val = &tail[val_start..i];
            if i < len {
                i += 1;
            }
            match attr_name.as_str() {
                "src" => src = val.to_string(),
                "alt" => alt = val.to_string(),
                _ => {}
            }
        }
    }

    (src, alt)
}

fn parse_class_attr(tail: &str) -> Vec<String> {
    let bytes = tail.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut classes = Vec::new();

    while i < len {
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len || bytes[i] == b'>' || bytes[i] == b'/' {
            break;
        }
        let name_start = i;
        while i < len
            && bytes[i] != b'='
            && bytes[i] != b' '
            && bytes[i] != b'>'
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let attr_name = &tail[name_start..i].to_lowercase();

        while i < len && (bytes[i] == b' ' || bytes[i] == b'=') {
            i += 1;
        }
        if i < len && (bytes[i] == b'"' || bytes[i] == b'\'') {
            let quote = bytes[i];
            i += 1;
            let val_start = i;
            while i < len && bytes[i] != quote {
                i += 1;
            }
            let val = &tail[val_start..i];
            if i < len {
                i += 1;
            }
            if attr_name == "class" {
                classes = val.split_whitespace().map(|s| s.to_string()).collect();
                break;
            }
        }
    }
    classes
}

pub fn parse_xhtml_with_css(html: &str) -> (Vec<ContentBlock>, Vec<CssRule>) {
    let mut rules = extract_embedded_css(html);
    let (prepared, inline_rules) = inject_inline_style_classes(html);
    rules.extend(inline_rules);
    (parse_xhtml(&prepared), rules)
}

fn extract_embedded_css(html: &str) -> Vec<CssRule> {
    let lower = html.to_lowercase();
    let mut rules = Vec::new();
    let mut offset = 0;
    while let Some(start) = lower[offset..].find("<style") {
        let start = offset + start;
        let Some(open_end) = lower[start..].find('>') else {
            break;
        };
        let content_start = start + open_end + 1;
        let Some(close) = lower[content_start..].find("</style>") else {
            break;
        };
        let content_end = content_start + close;
        rules.extend(css::parse_css(&html[content_start..content_end]).0);
        offset = content_end + "</style>".len();
    }
    rules
}

fn inject_inline_style_classes(html: &str) -> (String, Vec<CssRule>) {
    let mut output = String::with_capacity(html.len());
    let mut rules = Vec::new();
    let mut offset = 0;
    let mut index = 0;
    while let Some(relative_start) = html[offset..].find('<') {
        let start = offset + relative_start;
        output.push_str(&html[offset..start]);
        let Some(relative_end) = html[start..].find('>') else {
            output.push_str(&html[start..]);
            return (output, rules);
        };
        let end = start + relative_end + 1;
        let tag = &html[start..end];
        if !tag.starts_with("</")
            && !tag.starts_with("<!--")
            && !tag.starts_with("<?")
            && let Some(style) = attribute_value(tag, "style")
        {
            let class = format!("__folio_inline_{index}");
            index += 1;
            output.push_str(&inject_class(tag, &class));
            rules.push(CssRule {
                selector: format!(".{class}"),
                properties: css::parse_declarations(&style),
            });
        } else {
            output.push_str(tag);
        }
        offset = end;
    }
    output.push_str(&html[offset..]);
    (output, rules)
}

fn attribute_value(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let mut search = 0;
    while let Some(found) = lower[search..].find(name) {
        let start = search + found;
        let before_ok =
            start == 0 || !lower.as_bytes()[start.saturating_sub(1)].is_ascii_alphanumeric();
        let mut cursor = start + name.len();
        while cursor < tag.len() && tag.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if before_ok && tag.as_bytes().get(cursor) == Some(&b'=') {
            cursor += 1;
            while cursor < tag.len() && tag.as_bytes()[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            let quote = *tag.as_bytes().get(cursor)?;
            if quote == b'\'' || quote == b'"' {
                cursor += 1;
                let end = tag[cursor..].find(quote as char)? + cursor;
                return Some(tag[cursor..end].to_string());
            }
        }
        search = start + name.len();
    }
    None
}

fn inject_class(tag: &str, class: &str) -> String {
    if let Some(existing) = attribute_value(tag, "class") {
        let needle_double = format!("class=\"{existing}\"");
        let needle_single = format!("class='{existing}'");
        if tag.contains(&needle_double) {
            return tag.replacen(&needle_double, &format!("class=\"{existing} {class}\""), 1);
        }
        if tag.contains(&needle_single) {
            return tag.replacen(&needle_single, &format!("class='{existing} {class}'"), 1);
        }
    }
    let insertion = tag
        .rfind("/>")
        .or_else(|| tag.rfind('>'))
        .unwrap_or(tag.len());
    format!(
        "{} class=\"{}\"{}",
        &tag[..insertion],
        class,
        &tag[insertion..]
    )
}

pub fn parse_xhtml(html: &str) -> Vec<ContentBlock> {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut current_spans: Vec<InlineNode> = Vec::new();
    let mut text_buf = String::new();
    let mut bold_depth: u32 = 0;
    let mut italic_depth: u32 = 0;
    let mut in_heading: Option<u8> = None;
    let mut in_paragraph = false;
    let mut block_classes: Vec<String> = Vec::new();
    let mut current_span_classes: Vec<String> = Vec::new();
    let mut in_script = false;
    let mut in_style = false;
    let mut in_table = false;
    let mut table_rows: Vec<TableRow> = Vec::new();
    let mut current_row: Vec<TableCell> = Vec::new();
    let mut current_cell_header = false;
    let mut table_classes = Vec::new();
    let mut row_classes = Vec::new();
    let mut cell_classes = Vec::new();
    let mut in_list: Option<bool> = None;
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut list_classes = Vec::new();
    let mut list_depth = 0usize;
    let mut item_classes = Vec::new();
    let mut list_class_stack: Vec<Vec<String>> = Vec::new();

    let html = html
        .replace("<hr/>", "\n---\n")
        .replace("<hr />", "\n---\n")
        .replace("<hr>", "\n---\n");

    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;
    let mut iterations = 0u64;
    const MAX_ITERATIONS: u64 = 10_000_000;

    while pos < len {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            eprintln!(
                "[parse_xhtml] ERROR: exceeded {} iterations at pos={}/len={}",
                MAX_ITERATIONS, pos, len
            );
            break;
        }
        if iterations % 100_000 == 0 {
            eprintln!("[parse_xhtml] iteration {iterations}, pos={pos}/{len}");
        }
        if bytes[pos] == b'<' {
            pos += 1;
            if pos >= len {
                break;
            }

            let is_closing = bytes[pos] == b'/';
            if is_closing {
                pos += 1;
            }

            let tag_start = pos;
            while pos < len
                && bytes[pos] != b' '
                && bytes[pos] != b'>'
                && bytes[pos] != b'/'
                && bytes[pos] != b'\n'
            {
                pos += 1;
            }
            let tag_name = &html[tag_start..pos];
            let tag_lower = tag_name.to_lowercase();

            let attr_tail = &html[pos..];
            let tag_classes = if !is_closing {
                parse_class_attr(attr_tail)
            } else {
                Vec::new()
            };

            if !is_closing && tag_lower == "img" {
                let (src, alt) = parse_img_attributes(&html[pos..]);
                flush_text(
                    &mut text_buf,
                    &mut current_spans,
                    bold_depth,
                    italic_depth,
                    &current_span_classes,
                );
                if !src.is_empty() {
                    if in_heading.is_some() || in_paragraph {
                        current_spans.push(InlineNode::Image {
                            src,
                            alt,
                            classes: tag_classes,
                        });
                    } else {
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            block_classes.clone(),
                        );
                        blocks.push(ContentBlock::Image { src, alt });
                    }
                }

                while pos < len && bytes[pos] != b'>' {
                    pos += 1;
                }
                if pos < len {
                    pos += 1;
                }
                continue;
            }

            if !is_closing && tag_lower == "br" {
                flush_text(
                    &mut text_buf,
                    &mut current_spans,
                    bold_depth,
                    italic_depth,
                    &current_span_classes,
                );
                if in_heading.is_some() || in_paragraph {
                    current_spans.push(InlineNode::LineBreak);
                }
                while pos < len && bytes[pos] != b'>' {
                    pos += 1;
                }
                if pos < len {
                    pos += 1;
                }
                continue;
            }

            while pos < len && bytes[pos] != b'>' {
                pos += 1;
            }
            if pos < len {
                pos += 1;
            }

            if is_closing {
                match tag_lower.as_str() {
                    "td" | "th" if in_table => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        let text = inline_nodes_text(&current_spans);
                        current_spans.clear();
                        current_row.push(TableCell {
                            text: text.trim().to_string(),
                            header: current_cell_header,
                            classes: std::mem::take(&mut cell_classes),
                        });
                        in_paragraph = false;
                    }
                    "tr" if in_table => {
                        text_buf.clear();
                        if !current_row.is_empty() {
                            table_rows.push(TableRow {
                                cells: std::mem::take(&mut current_row),
                                classes: std::mem::take(&mut row_classes),
                            });
                        }
                    }
                    "table" if in_table => {
                        text_buf.clear();
                        if !current_row.is_empty() {
                            table_rows.push(TableRow {
                                cells: std::mem::take(&mut current_row),
                                classes: std::mem::take(&mut row_classes),
                            });
                        }
                        if !table_rows.is_empty() {
                            blocks.push(ContentBlock::Table {
                                rows: std::mem::take(&mut table_rows),
                                classes: std::mem::take(&mut table_classes),
                            });
                        }
                        in_table = false;
                    }
                    "li" if in_list.is_some() => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        let spans = take_text_spans(&mut current_spans);
                        if !spans.is_empty() {
                            list_items.push(ListItem {
                                spans,
                                classes: std::mem::take(&mut item_classes),
                                depth: list_depth,
                            });
                        }
                        in_paragraph = false;
                    }
                    "ul" | "ol" if in_list.is_some() && list_depth > 0 => {
                        text_buf.clear();
                        list_depth -= 1;
                        list_class_stack.pop();
                    }
                    "ul" | "ol" if in_list.is_some() => {
                        text_buf.clear();
                        let ordered = in_list.take().unwrap_or(false);
                        if !list_items.is_empty() {
                            blocks.push(ContentBlock::List {
                                ordered,
                                items: std::mem::take(&mut list_items),
                                classes: std::mem::take(&mut list_classes),
                            });
                        }
                    }
                    "script" => in_script = false,
                    "style" => in_style = false,
                    "title" => in_style = false,
                    "strong" | "b" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        bold_depth = bold_depth.saturating_sub(1);
                    }
                    "em" | "i" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        italic_depth = italic_depth.saturating_sub(1);
                    }
                    "small" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        current_span_classes.retain(|class| class != "__folio-small");
                    }
                    "span" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        current_span_classes.clear();
                    }
                    "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "section"
                    | "article" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        in_heading = None;
                        in_paragraph = false;
                    }
                    _ => {}
                }
            } else {
                match tag_lower.as_str() {
                    "table" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        in_heading = None;
                        in_paragraph = false;
                        in_table = true;
                        table_classes = tag_classes;
                        table_rows.clear();
                        current_row.clear();
                    }
                    "thead" | "tbody" if in_table => text_buf.clear(),
                    "tr" if in_table => {
                        text_buf.clear();
                        current_row.clear();
                        row_classes = tag_classes;
                        if table_rows.len() % 2 == 1 {
                            row_classes.push("__folio-even".to_string());
                        }
                    }
                    "td" | "th" if in_table => {
                        text_buf.clear();
                        current_spans.clear();
                        current_cell_header = tag_lower == "th";
                        cell_classes = tag_classes;
                        in_paragraph = true;
                    }
                    "ul" | "ol" if in_list.is_some() => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        let spans = take_text_spans(&mut current_spans);
                        if !spans.is_empty() {
                            list_items.push(ListItem {
                                spans,
                                classes: std::mem::take(&mut item_classes),
                                depth: list_depth,
                            });
                        }
                        list_depth += 1;
                        list_class_stack.push(tag_classes);
                    }
                    "ul" | "ol" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        in_heading = None;
                        in_paragraph = false;
                        in_list = Some(tag_lower == "ol");
                        list_classes = tag_classes;
                        list_class_stack = vec![list_classes.clone()];
                        list_depth = 0;
                        list_items.clear();
                    }
                    "li" if in_list.is_some() => {
                        text_buf.clear();
                        current_spans.clear();
                        item_classes = tag_classes;
                        if let Some(active_classes) = list_class_stack.last() {
                            item_classes.extend(active_classes.iter().cloned());
                        }
                        in_paragraph = true;
                    }
                    "script" => in_script = true,
                    "style" => in_style = true,
                    "title" => in_style = true,
                    "strong" | "b" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        bold_depth += 1;
                    }
                    "em" | "i" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        italic_depth += 1;
                    }
                    "small" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        if !current_span_classes
                            .iter()
                            .any(|class| class == "__folio-small")
                        {
                            current_span_classes.push("__folio-small".to_string());
                        }
                    }
                    "span" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        current_span_classes = tag_classes;
                    }
                    "p" | "div" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_paragraph = true;
                    }
                    "h1" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(1);
                    }
                    "h2" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(2);
                    }
                    "h3" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(3);
                    }
                    "h4" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(4);
                    }
                    "h5" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(5);
                    }
                    "h6" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                        in_heading = Some(6);
                    }
                    "section" | "article" => {
                        flush_text(
                            &mut text_buf,
                            &mut current_spans,
                            bold_depth,
                            italic_depth,
                            &current_span_classes,
                        );
                        push_block(
                            &mut blocks,
                            &mut current_spans,
                            in_heading,
                            in_paragraph,
                            std::mem::take(&mut block_classes),
                        );
                        block_classes = tag_classes;
                    }
                    _ => {}
                }
            }
        } else if in_script || in_style {
            while pos < len && bytes[pos] != b'<' {
                pos += 1;
            }
        } else {
            let start = pos;
            while pos < len && bytes[pos] != b'<' {
                pos += 1;
            }
            let text = &html[start..pos];
            let decoded = decode_entities(text);
            text_buf.push_str(&decoded);
        }
    }

    flush_text(
        &mut text_buf,
        &mut current_spans,
        bold_depth,
        italic_depth,
        &current_span_classes,
    );
    push_block(
        &mut blocks,
        &mut current_spans,
        in_heading,
        in_paragraph,
        std::mem::take(&mut block_classes),
    );

    blocks
}

fn flush_text(
    text_buf: &mut String,
    spans: &mut Vec<InlineNode>,
    bold_depth: u32,
    italic_depth: u32,
    span_classes: &[String],
) {
    let collapsed = collapse_html_whitespace(text_buf);
    text_buf.clear();
    if collapsed.is_empty() {
        return;
    }

    spans.push(InlineNode::Text(StyledSpan {
        text: collapsed,
        bold: bold_depth > 0,
        italic: italic_depth > 0,
        underline: false,
        strikethrough: false,
        color: None,
        size: None,
        link: None,
        classes: span_classes.to_vec(),
    }));
}

fn collapse_html_whitespace(text: &str) -> String {
    let starts_with_space = text.chars().next().is_some_and(char::is_whitespace);
    let ends_with_space = text.chars().next_back().is_some_and(char::is_whitespace);
    let words: Vec<_> = text.split_whitespace().collect();

    if words.is_empty() {
        return if starts_with_space {
            " ".to_string()
        } else {
            String::new()
        };
    }

    let mut collapsed = words.join(" ");
    if starts_with_space {
        collapsed.insert(0, ' ');
    }
    if ends_with_space {
        collapsed.push(' ');
    }
    collapsed
}

fn inline_nodes_text(nodes: &[InlineNode]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            InlineNode::Text(span) => span.text.as_str(),
            InlineNode::LineBreak => "\n",
            InlineNode::Image { alt, .. } => alt.as_str(),
        })
        .collect()
}

fn take_text_spans(nodes: &mut Vec<InlineNode>) -> Vec<StyledSpan> {
    std::mem::take(nodes)
        .into_iter()
        .filter_map(|node| match node {
            InlineNode::Text(span) => Some(span),
            InlineNode::LineBreak | InlineNode::Image { .. } => None,
        })
        .collect()
}

fn push_block(
    blocks: &mut Vec<ContentBlock>,
    spans: &mut Vec<InlineNode>,
    heading: Option<u8>,
    _in_paragraph: bool,
    classes: Vec<String>,
) {
    if spans.is_empty() {
        return;
    }

    if let Some(InlineNode::Text(first)) = spans.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(InlineNode::Text(last)) = spans.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    spans.retain(|node| !matches!(node, InlineNode::Text(span) if span.text.is_empty()));
    if spans.is_empty() {
        return;
    }

    let taken = std::mem::take(spans);
    if taken
        .iter()
        .any(|node| !matches!(node, InlineNode::Text(_)))
    {
        blocks.push(ContentBlock::Inline {
            kind: heading.map_or(InlineBlockKind::Paragraph, InlineBlockKind::Heading),
            nodes: taken,
            classes,
        });
        return;
    }
    let taken: Vec<StyledSpan> = taken
        .into_iter()
        .filter_map(|node| match node {
            InlineNode::Text(span) => Some(span),
            InlineNode::LineBreak => None,
            InlineNode::Image { .. } => None,
        })
        .collect();
    if let Some(level) = heading {
        blocks.push(ContentBlock::Heading {
            level,
            spans: taken,
            classes,
        });
    } else {
        blocks.push(ContentBlock::Paragraph {
            spans: taken,
            classes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_preserves_block_classes() {
        let html = r#"<h1 class="ft1">Título</h1><p><span class="ft2">Hola</span> mundo</p>"#;
        let blocks = parse_xhtml(html);
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            ContentBlock::Heading {
                level,
                spans,
                classes,
            } => {
                assert_eq!(*level, 1);
                assert_eq!(classes, &["ft1"]);
                assert_eq!(spans.len(), 1);
                assert_eq!(spans[0].text, "Título");
            }
            _ => panic!("expected heading"),
        }
        match &blocks[1] {
            ContentBlock::Paragraph { spans, classes } => {
                assert!(classes.is_empty());
                assert_eq!(spans.len(), 2);
                assert_eq!(spans[0].classes, &["ft2"]);
                assert_eq!(spans[0].text, "Hola");
                assert_eq!(spans[1].text, " mundo");
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn image_inside_heading_is_preserved_in_inline_order() {
        let html = r#"<h2 class="litos">CAPÍTULO 1<img class="fighter" src="title.jpg" />DUQUE TIAGO</h2>"#;
        let blocks = parse_xhtml(html);

        assert_eq!(blocks.len(), 1);
        let ContentBlock::Inline {
            kind: InlineBlockKind::Heading(2),
            nodes,
            classes,
        } = &blocks[0]
        else {
            panic!("expected mixed inline heading");
        };
        assert_eq!(classes, &["litos"]);
        assert!(matches!(&nodes[0], InlineNode::Text(span) if span.text == "CAPÍTULO 1"));
        assert!(
            matches!(&nodes[1], InlineNode::Image { src, classes, .. } if src == "title.jpg" && classes == &["fighter"])
        );
        assert!(matches!(&nodes[2], InlineNode::Text(span) if span.text == "DUQUE TIAGO"));
    }

    #[test]
    fn heading_preserves_small_text_and_explicit_line_break() {
        let html = r#"<h1 title="Lo que ha pasado antes"><small>LO QUE HA PASADO ANTES</small> <br/><span class="stt"><small>&#160;</small></span></h1>"#;
        let blocks = parse_xhtml(html);

        assert_eq!(blocks.len(), 1);
        let ContentBlock::Inline {
            kind: InlineBlockKind::Heading(1),
            nodes,
            ..
        } = &blocks[0]
        else {
            panic!("expected inline heading");
        };
        assert!(matches!(&nodes[0], InlineNode::Text(span)
            if span.text == "LO QUE HA PASADO ANTES"
                && span.classes.iter().any(|class| class == "__folio-small")));
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, InlineNode::LineBreak))
        );
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, InlineNode::Text(span)
            if span.classes.iter().any(|class| class == "stt")
                && span.classes.iter().any(|class| class == "__folio-small")
                && !span.text.contains("160")))
        );
    }

    #[test]
    fn decodes_decimal_and_hexadecimal_numeric_entities() {
        assert_eq!(decode_entities("A&#160;B"), "A B");
        assert_eq!(decode_entities("&#x41;&#X42;"), "AB");
        assert_eq!(decode_entities("&#225;"), "á");
    }

    #[test]
    fn parses_tables_and_bulleted_lists_as_structured_blocks() {
        let html = r#"
            <h1 class="center">Capítulo 2</h1>
            <table><thead><tr>
                <th>Columna 1</th><th>Columna 2</th><th>Columna 3</th><th>Columna 4</th>
            </tr></thead><tbody>
                <tr><td>1-1</td><td>1-2</td><td>1-3</td><td>1-4</td></tr>
                <tr><td>2-1</td><td>2-2</td><td>2-3</td><td>2-4</td></tr>
                <tr><td>3-1</td><td>3-2</td><td>3-3</td><td>3-4</td></tr>
            </tbody></table>
            <ul><li>Uno</li><li>Dos</li><li>Tres</li><li>Cuatro</li><li>Cinco</li></ul>
        "#;
        let blocks = parse_xhtml(html);

        let table = blocks
            .iter()
            .find_map(|block| match block {
                ContentBlock::Table { rows, .. } => Some(rows),
                _ => None,
            })
            .expect("table block");
        assert_eq!(table.len(), 4);
        assert!(table.iter().all(|row| row.cells.len() == 4));
        assert!(table[0].cells.iter().all(|cell| cell.header));

        let items = blocks
            .iter()
            .find_map(|block| match block {
                ContentBlock::List { ordered, items, .. } if !ordered => Some(items),
                _ => None,
            })
            .expect("unordered list block");
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn extracts_embedded_and_inline_css_without_rendering_style_text() {
        let html = r#"
            <html><head><style>.rojo { color: #e74c3c; }</style></head>
            <body><h1 class="rojo" style="padding: 10px;">Título</h1></body></html>
        "#;
        let (blocks, rules) = parse_xhtml_with_css(html);

        assert_eq!(blocks.len(), 1);
        let ContentBlock::Heading { spans, classes, .. } = &blocks[0] else {
            panic!("expected heading");
        };
        assert_eq!(spans[0].text, "Título");
        assert!(classes.iter().any(|class| class == "rojo"));
        assert!(
            classes
                .iter()
                .any(|class| class.starts_with("__folio_inline_"))
        );
        assert!(rules.iter().any(|rule| rule.selector == ".rojo"));
        assert!(rules.iter().any(|rule| {
            rule.selector.starts_with(".__folio_inline_")
                && rule
                    .properties
                    .get("padding")
                    .is_some_and(|value| value == "10px")
        }));
    }

    #[test]
    fn parse_preserves_spaces_between_mixed_inline_styles() {
        let html = "<p>Hola <strong>mundo</strong>, cómo va.</p>";
        let blocks = parse_xhtml(html);

        let ContentBlock::Paragraph { spans, .. } = &blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "Hola mundo, cómo va."
        );
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Hola ");
        assert!(spans[1].bold);
        assert_eq!(spans[1].text, "mundo");
        assert_eq!(spans[2].text, ", cómo va.");
    }
}
