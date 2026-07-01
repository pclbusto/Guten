use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct CssRule {
    pub selector: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct FontFaceRule {
    pub family: String,
    pub src: String,
    pub weight: u16,
    pub italic: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<cosmic::iced::Color>,
    pub align: Option<cosmic::iced::alignment::Horizontal>,
}

pub fn parse_css(css: &str) -> (Vec<CssRule>, Vec<FontFaceRule>) {
    let mut rules = Vec::new();
    let mut font_faces = Vec::new();
    let css = strip_comments(css);
    let mut pos = 0;
    let bytes = css.as_bytes();
    let len = bytes.len();

    while pos < len {
        skip_whitespace(bytes, &mut pos, len);
        if pos >= len {
            break;
        }

        if bytes[pos] == b'@' {
            if let Some(face) = parse_font_face(bytes, &css, &mut pos, len) {
                font_faces.push(face);
            } else {
                skip_at_rule(bytes, &mut pos, len);
            }
            continue;
        }

        let sel_start = pos;
        while pos < len && bytes[pos] != b'{' {
            pos += 1;
        }
        let selector = css[sel_start..pos].trim().to_string();
        if pos >= len {
            break;
        }
        pos += 1;

        let props_start = pos;
        let mut depth = 1;
        while pos < len && depth > 0 {
            if bytes[pos] == b'{' {
                depth += 1;
            } else if bytes[pos] == b'}' {
                depth -= 1;
            }
            if depth > 0 {
                pos += 1;
            }
        }
        let props_str = &css[props_start..pos];
        if pos < len {
            pos += 1;
        }

        let properties = parse_properties(props_str);
        if !properties.is_empty() && !selector.is_empty() {
            eprintln!(
                "[css] Parsed rule: {} -> {} properties",
                selector,
                properties.len()
            );
            rules.push(CssRule {
                selector,
                properties,
            });
        }
    }

    eprintln!("[css] Total rules parsed: {}", rules.len());
    (rules, font_faces)
}

fn parse_font_face(bytes: &[u8], css: &str, pos: &mut usize, len: usize) -> Option<FontFaceRule> {
    let start = *pos;
    *pos += 1; // skip '@'
    while *pos < len && !bytes[*pos].is_ascii_whitespace() && bytes[*pos] != b'{' {
        *pos += 1;
    }
    let keyword = css[start..*pos].to_lowercase();
    if keyword != "@font-face" {
        *pos = start;
        return None;
    }

    skip_whitespace(bytes, pos, len);
    if *pos >= len || bytes[*pos] != b'{' {
        return None;
    }
    *pos += 1;

    let props_start = *pos;
    let mut depth = 1;
    while *pos < len && depth > 0 {
        if bytes[*pos] == b'{' {
            depth += 1;
        } else if bytes[*pos] == b'}' {
            depth -= 1;
        }
        if depth > 0 {
            *pos += 1;
        }
    }
    let props_str = &css[props_start..*pos];
    if *pos < len {
        *pos += 1;
    }

    let props = parse_properties(props_str);
    let family = props.get("font-family")?;
    let src = props.get("src")?;
    let weight = props
        .get("font-weight")
        .map(|v| parse_font_weight(v))
        .unwrap_or(400);
    let italic = props
        .get("font-style")
        .map(|v| v.eq_ignore_ascii_case("italic") || v.eq_ignore_ascii_case("oblique"))
        .unwrap_or(false);
    Some(FontFaceRule {
        family: clean_font_family(family),
        src: clean_src(src),
        weight,
        italic,
    })
}

fn parse_font_weight(value: &str) -> u16 {
    let value = value.trim();
    match value {
        "normal" => 400,
        "bold" => 700,
        "bolder" => 700,
        "lighter" => 300,
        _ => value.parse().unwrap_or(400),
    }
}

fn clean_font_family(value: &str) -> String {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches(|c: char| c == '\'' || c == '"')
        .to_string()
}

fn clean_src(value: &str) -> String {
    let value = value.trim();
    let value = value.strip_prefix("url(").unwrap_or(value);
    let value = value.strip_suffix(")").unwrap_or(value);
    value
        .trim()
        .trim_matches(|c: char| c == '\'' || c == '"')
        .to_string()
}

fn strip_comments(css: &str) -> String {
    let mut result = String::with_capacity(css.len());
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

fn parse_properties(block: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();
    for part in block.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((key, value)) = part.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            if !key.is_empty() && !value.is_empty() {
                props.insert(key, value);
            }
        }
    }
    props
}

fn skip_whitespace(bytes: &[u8], pos: &mut usize, len: usize) {
    while *pos < len && bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

fn skip_at_rule(bytes: &[u8], pos: &mut usize, len: usize) {
    while *pos < len {
        if bytes[*pos] == b'{' {
            let mut depth = 1;
            *pos += 1;
            while *pos < len && depth > 0 {
                if bytes[*pos] == b'{' {
                    depth += 1;
                } else if bytes[*pos] == b'}' {
                    depth -= 1;
                }
                *pos += 1;
            }
            return;
        } else if bytes[*pos] == b';' {
            *pos += 1;
            return;
        }
        *pos += 1;
    }
}

pub fn resolve_style(tag: &str, classes: &[String], rules: &[CssRule]) -> ResolvedStyle {
    let mut style = ResolvedStyle::default();
    let tag_lower = tag.to_lowercase();

    if !classes.is_empty() {
        eprintln!(
            "[css] Resolving style for tag='{}' classes={:?}",
            tag, classes
        );
    }

    for rule in rules {
        let applies = does_selector_match(&rule.selector, &tag_lower, classes);
        if !applies {
            continue;
        }

        if !classes.is_empty() {
            eprintln!("[css]   Matched rule: {}", rule.selector);
        }

        for (prop, value) in &rule.properties {
            apply_property(&mut style, prop, value);
        }
    }

    if !classes.is_empty() {
        eprintln!("[css]   Resolved font_family={:?}", style.font_family);
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[ignore]
    fn test_parse_style_css() {
        let css_text = fs::read_to_string("/home/pedro/Documentos/style.css").unwrap();
        println!("\n=== Parsing CSS ===");
        let (rules, _font_faces) = parse_css(&css_text);

        println!("\n=== Resolving styles ===");

        // Test h1 with class ft1
        println!("\nTesting <h1 class='ft1'>:");
        let style = resolve_style("h1", &["ft1".to_string()], &rules);
        println!("  font_family: {:?}", style.font_family);
        println!("  font_size: {:?}", style.font_size);

        // Test span with class ft2
        println!("\nTesting <span class='ft2'>:");
        let style = resolve_style("span", &["ft2".to_string()], &rules);
        println!("  font_family: {:?}", style.font_family);

        // Test p with class ftq
        println!("\nTesting <p class='ftq'>:");
        let style = resolve_style("p", &["ftq".to_string()], &rules);
        println!("  font_family: {:?}", style.font_family);

        // Test p without class
        println!("\nTesting <p> (no class):");
        let style = resolve_style("p", &[], &rules);
        println!("  font_family: {:?}", style.font_family);
    }

    #[test]
    fn parse_font_face_rules() {
        let css = r#"
            @font-face { font-family: "Nikona"; src: url(../Fonts/Nikona.otf); }
            @font-face { font-family: "Nikona-B"; src: url('../Fonts/Nikona-B.otf'); font-weight: bold; }
            @font-face { font-family: "DIN"; src: url(../Fonts/DIN.otf); font-style: italic; }
        "#;
        let (rules, faces) = parse_css(css);
        assert!(rules.is_empty());
        assert_eq!(faces.len(), 3);
        assert_eq!(faces[0].family, "Nikona");
        assert_eq!(faces[0].src, "../Fonts/Nikona.otf");
        assert_eq!(faces[0].weight, 400);
        assert!(!faces[0].italic);
        assert_eq!(faces[1].family, "Nikona-B");
        assert_eq!(faces[1].src, "../Fonts/Nikona-B.otf");
        assert_eq!(faces[1].weight, 700);
        assert!(!faces[1].italic);
        assert_eq!(faces[2].family, "DIN");
        assert_eq!(faces[2].src, "../Fonts/DIN.otf");
        assert!(faces[2].italic);
    }

    #[test]
    fn descendant_selectors_do_not_match_without_ancestor_context() {
        let css = r#"
            b, strong, .negrita, negrita p { font-weight: bold; }
            .ftq { font-family: "DINNextLTPro"; }
        "#;
        let (rules, _) = parse_css(css);

        let p = resolve_style("p", &[], &rules);
        assert!(!p.bold, "plain paragraphs must not match `negrita p`");

        let ftq = resolve_style("p", &["ftq".to_string()], &rules);
        assert_eq!(ftq.font_family.as_deref(), Some("DINNextLTPro"));
        assert!(!ftq.bold, ".ftq must not become bold through `negrita p`");
    }

    #[test]
    fn simple_class_and_tag_selectors_still_match() {
        let css = r#"
            h1 { font-weight: bold; }
            .ft1 { font-family: "Nikona"; }
        "#;
        let (rules, _) = parse_css(css);

        let h1 = resolve_style("h1", &["ft1".to_string()], &rules);
        assert!(h1.bold);
        assert_eq!(h1.font_family.as_deref(), Some("Nikona"));
    }
}

fn does_selector_match(selector: &str, tag: &str, classes: &[String]) -> bool {
    for part in selector.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if selector_part_matches(part, tag, classes) {
            return true;
        }
    }
    false
}

fn selector_part_matches(part: &str, tag: &str, classes: &[String]) -> bool {
    let part = part.trim();
    if part.contains(char::is_whitespace) {
        return false;
    }

    if part == tag || part == "*" {
        return true;
    }

    if part.starts_with('.') {
        let class_name = &part[1..];
        return classes.iter().any(|c| c == class_name);
    }

    if let Some((sel_tag, sel_class)) = part.split_once('.') {
        if sel_tag != tag && sel_tag != "*" {
            return false;
        }
        return classes.iter().any(|c| c == sel_class);
    }

    false
}

fn apply_property(style: &mut ResolvedStyle, prop: &str, value: &str) {
    match prop {
        "font-family" => {
            let family = value
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c: char| c == '\'' || c == '"' || c == '"')
                .to_string();
            if !family.is_empty() {
                style.font_family = Some(family);
            }
        }
        "font-size" => {
            if let Some(pt) = parse_font_size(value) {
                style.font_size = Some(pt);
            }
        }
        "font-weight" => {
            style.bold = value == "bold"
                || value == "bolder"
                || value == "700"
                || value == "800"
                || value == "900";
        }
        "font-style" => {
            style.italic = value == "italic" || value == "oblique";
        }
        "text-decoration" => {
            style.underline = value.contains("underline");
        }
        "color" => {
            style.color = parse_css_color(value);
        }
        "text-align" => {
            style.align = match value {
                "left" => Some(cosmic::iced::alignment::Horizontal::Left),
                "right" => Some(cosmic::iced::alignment::Horizontal::Right),
                "center" => Some(cosmic::iced::alignment::Horizontal::Center),
                "justify" => Some(cosmic::iced::alignment::Horizontal::Left),
                _ => None,
            };
        }
        _ => {}
    }
}

fn parse_font_size(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.ends_with("pt") {
        value[..value.len() - 2].trim().parse().ok()
    } else if value.ends_with("px") {
        let px: f32 = value[..value.len() - 2].trim().parse().ok()?;
        Some(px * 0.75)
    } else if value.ends_with("em") {
        let em: f32 = value[..value.len() - 2].trim().parse().ok()?;
        Some(em * 12.0)
    } else if value.ends_with("rem") {
        let rem: f32 = value[..value.len() - 3].trim().parse().ok()?;
        Some(rem * 14.0)
    } else {
        value.parse().ok()
    }
}

fn parse_css_color(value: &str) -> Option<cosmic::iced::Color> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(cosmic::iced::Color::from_rgb8(r, g, b));
        }
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(cosmic::iced::Color::from_rgb8(r, g, b));
        }
    }
    if value.starts_with("rgb") {
        if let Some(inner) = value.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() == 3 {
                let r: u8 = parts[0].trim().parse().ok()?;
                let g: u8 = parts[1].trim().parse().ok()?;
                let b: u8 = parts[2].trim().parse().ok()?;
                return Some(cosmic::iced::Color::from_rgb8(r, g, b));
            }
        }
    }
    None
}
