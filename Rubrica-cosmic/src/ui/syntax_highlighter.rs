use cosmic::iced::core::text::highlighter::Format;
use cosmic::iced::widget::text::Highlighter;
use cosmic::iced::{Color, Font};
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    Html,
    Css,
}

#[derive(Clone, Copy, Debug)]
pub enum TokenKind {
    Tag,
    Attribute,
    String,
    Comment,
    Selector,
    Property,
    Value,
    Number,
    Plain,
}

pub struct SyntaxHighlighter {
    language: Language,
    current_line: usize,
}

impl Highlighter for SyntaxHighlighter {
    type Settings = Language;
    type Highlight = TokenKind;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, TokenKind)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            language: *settings,
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.language = *new_settings;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let mut tokens = Vec::new();
        match self.language {
            Language::Html => highlight_html(line, &mut tokens),
            Language::Css => highlight_css(line, &mut tokens),
        }
        tokens.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

pub fn token_format(kind: &TokenKind, theme: &cosmic::Theme) -> Format<Font> {
    let is_dark = theme.theme_type.is_dark();
    let color = match kind {
        TokenKind::Tag => {
            if is_dark {
                Color::from_rgb8(0x66, 0xb3, 0xff)
            } else {
                Color::from_rgb8(0x22, 0x88, 0xff)
            }
        }
        TokenKind::Attribute => {
            if is_dark {
                Color::from_rgb8(0xff, 0xaa, 0x66)
            } else {
                Color::from_rgb8(0xdd, 0x88, 0x44)
            }
        }
        TokenKind::String => {
            if is_dark {
                Color::from_rgb8(0x55, 0xcc, 0x88)
            } else {
                Color::from_rgb8(0x22, 0xaa, 0x66)
            }
        }
        TokenKind::Comment => {
            if is_dark {
                Color::from_rgb8(0xaa, 0xaa, 0xaa)
            } else {
                Color::from_rgb8(0x88, 0x88, 0x88)
            }
        }
        TokenKind::Selector => {
            if is_dark {
                Color::from_rgb8(0x77, 0xaa, 0xff)
            } else {
                Color::from_rgb8(0x44, 0x88, 0xdd)
            }
        }
        TokenKind::Property => {
            if is_dark {
                Color::from_rgb8(0xff, 0x88, 0x66)
            } else {
                Color::from_rgb8(0xdd, 0x66, 0x44)
            }
        }
        TokenKind::Value => {
            if is_dark {
                Color::from_rgb8(0xcc, 0xcc, 0xcc)
            } else {
                Color::from_rgb8(0xaa, 0xaa, 0xaa)
            }
        }
        TokenKind::Number => {
            if is_dark {
                Color::from_rgb8(0x99, 0xee, 0xff)
            } else {
                Color::from_rgb8(0x66, 0xdd, 0xee)
            }
        }
        TokenKind::Plain => {
            return Format {
                color: None,
                font: None,
            };
        }
    };
    Format {
        color: Some(color),
        font: None,
    }
}

fn highlight_html(line: &str, tokens: &mut Vec<(Range<usize>, TokenKind)>) {
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if i + 4 <= bytes.len() && &bytes[i..i + 4] == b"<!--" {
                let start = i;
                if let Some(end) = line[i..].find("-->") {
                    i += end + 3;
                } else {
                    i = bytes.len();
                }
                tokens.push((start..i, TokenKind::Comment));
                continue;
            }

            let tag_start = i;
            let mut j = i + 1;
            let mut quote: Option<u8> = None;
            while j < bytes.len() {
                let c = bytes[j];
                if let Some(q) = quote {
                    if c == q {
                        quote = None;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = Some(c);
                } else if c == b'>' {
                    j += 1;
                    break;
                }
                j += 1;
            }

            let tag_content = &line[tag_start..j];
            let mut k = 1;
            if tag_content.as_bytes().get(k) == Some(&b'/') {
                k += 1;
            }
            while k < tag_content.len() && tag_content.as_bytes()[k].is_ascii_whitespace() {
                k += 1;
            }
            let name_start = k;
            while k < tag_content.len()
                && !tag_content.as_bytes()[k].is_ascii_whitespace()
                && tag_content.as_bytes()[k] != b'>'
            {
                k += 1;
            }
            if name_start < k {
                tokens.push((tag_start + name_start..tag_start + k, TokenKind::Tag));
            }

            let mut expect_attr = true;
            while k < tag_content.len() {
                let c = tag_content.as_bytes()[k];
                if c.is_ascii_whitespace() || c == b'>' || c == b'/' {
                    k += 1;
                    continue;
                }
                if c == b'"' || c == b'\'' {
                    let str_start = k;
                    let quote = c;
                    k += 1;
                    while k < tag_content.len() && tag_content.as_bytes()[k] != quote {
                        k += 1;
                    }
                    if k < tag_content.len() {
                        k += 1;
                    }
                    tokens.push((tag_start + str_start..tag_start + k, TokenKind::String));
                    expect_attr = true;
                    continue;
                }
                if expect_attr {
                    let attr_start = k;
                    while k < tag_content.len()
                        && !tag_content.as_bytes()[k].is_ascii_whitespace()
                        && tag_content.as_bytes()[k] != b'='
                        && tag_content.as_bytes()[k] != b'>'
                        && tag_content.as_bytes()[k] != b'/'
                    {
                        k += 1;
                    }
                    tokens.push((tag_start + attr_start..tag_start + k, TokenKind::Attribute));
                    if k < tag_content.len() && tag_content.as_bytes()[k] == b'=' {
                        k += 1;
                    }
                    expect_attr = false;
                } else {
                    k += 1;
                }
            }

            i = j;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            tokens.push((start..i, TokenKind::Plain));
        }
    }
}

fn highlight_css(line: &str, tokens: &mut Vec<(Range<usize>, TokenKind)>) {
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        if i + 2 <= bytes.len() && &bytes[i..i + 2] == b"/*" {
            let start = i;
            if let Some(end) = line[i..].find("*/") {
                i += end + 2;
            } else {
                i = bytes.len();
            }
            tokens.push((start..i, TokenKind::Comment));
            continue;
        }

        if bytes[i] == b'{' || bytes[i] == b'}' {
            i += 1;
            continue;
        }

        if let Some(colon) = line[i..].find(':') {
            let colon_abs = i + colon;
            if let Some(brace_open) = line[i..colon_abs].find('{') {
                let brace_abs = i + brace_open;
                if brace_abs > i {
                    tokens.push((i..brace_abs, TokenKind::Selector));
                }
                i = brace_abs + 1;
                continue;
            }
            if colon_abs > i {
                tokens.push((i..colon_abs, TokenKind::Property));
            }
            let val_start = colon_abs + 1;
            let val_end = if let Some(semi) = line[val_start..].find(';') {
                val_start + semi
            } else {
                bytes.len()
            };
            push_css_value_tokens(&line[val_start..val_end], val_start, tokens);
            i = val_end;
            if i < bytes.len() && bytes[i] == b';' {
                i += 1;
            }
        } else if let Some(brace) = line[i..].find('{') {
            let brace_abs = i + brace;
            if brace_abs > i {
                tokens.push((i..brace_abs, TokenKind::Selector));
            }
            i = brace_abs + 1;
        } else {
            tokens.push((i..bytes.len(), TokenKind::Plain));
            break;
        }
    }
}

fn push_css_value_tokens(value: &str, offset: usize, tokens: &mut Vec<(Range<usize>, TokenKind)>) {
    let mut i = 0;
    let bytes = value.as_bytes();
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'"' || c == b'\'' {
            let quote = c;
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            tokens.push((offset + start..offset + i, TokenKind::String));
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'%' {
                i += 1;
            }
            tokens.push((offset + start..offset + i, TokenKind::Number));
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\'' && !bytes[i].is_ascii_digit()
        {
            i += 1;
        }
        if start < i {
            tokens.push((offset + start..offset + i, TokenKind::Value));
        }
    }
}
