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
    text.replace("&amp;", "&")
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

pub fn parse_xhtml(html: &str) -> Vec<ContentBlock> {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut current_spans: Vec<StyledSpan> = Vec::new();
    let mut text_buf = String::new();
    let mut bold_depth: u32 = 0;
    let mut italic_depth: u32 = 0;
    let mut in_heading: Option<u8> = None;
    let mut in_paragraph = false;
    let mut block_classes: Vec<String> = Vec::new();
    let mut current_span_classes: Vec<String> = Vec::new();
    let mut in_script = false;
    let mut in_style = false;

    let html = html
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n")
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
                push_block(
                    &mut blocks,
                    &mut current_spans,
                    in_heading,
                    in_paragraph,
                    block_classes.clone(),
                );

                if !src.is_empty() {
                    blocks.push(ContentBlock::Image { src, alt });
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
    spans: &mut Vec<StyledSpan>,
    bold_depth: u32,
    italic_depth: u32,
    span_classes: &[String],
) {
    let collapsed = collapse_html_whitespace(text_buf);
    text_buf.clear();
    if collapsed.is_empty() {
        return;
    }

    spans.push(StyledSpan {
        text: collapsed,
        bold: bold_depth > 0,
        italic: italic_depth > 0,
        underline: false,
        strikethrough: false,
        color: None,
        size: None,
        link: None,
        classes: span_classes.to_vec(),
    });
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

fn push_block(
    blocks: &mut Vec<ContentBlock>,
    spans: &mut Vec<StyledSpan>,
    heading: Option<u8>,
    _in_paragraph: bool,
    classes: Vec<String>,
) {
    if spans.is_empty() {
        return;
    }

    if let Some(first) = spans.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = spans.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    spans.retain(|span| !span.text.is_empty());
    if spans.is_empty() {
        return;
    }

    let taken = std::mem::take(spans);
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
    fn image_inside_heading_preserves_classes_on_both_text_blocks() {
        let html = r#"<h2 class="litos">CAPÍTULO 1<img src="title.jpg" />DUQUE TIAGO</h2>"#;
        let blocks = parse_xhtml(html);

        assert_eq!(blocks.len(), 3);
        for index in [0, 2] {
            let ContentBlock::Heading {
                level,
                spans,
                classes,
            } = &blocks[index]
            else {
                panic!("expected heading at index {index}");
            };
            assert_eq!(*level, 2);
            assert_eq!(classes, &["litos"]);
            assert_eq!(spans.len(), 1);
        }
        let heading_text = |block: &ContentBlock| match block {
            ContentBlock::Heading { spans, .. } => spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            _ => panic!("expected heading"),
        };
        assert_eq!(heading_text(&blocks[0]), "CAPÍTULO 1");
        assert!(matches!(&blocks[1], ContentBlock::Image { src, .. } if src == "title.jpg"));
        assert_eq!(heading_text(&blocks[2]), "DUQUE TIAGO");
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
