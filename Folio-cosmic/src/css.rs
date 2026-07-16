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
    pub float_left: bool,
    pub margin_right: f32,
    pub width: Option<f32>,
    pub width_percent: Option<f32>,
    pub height: Option<f32>,
    pub vertical_middle: bool,
    pub background_color: Option<cosmic::iced::Color>,
    pub gradient: Option<(cosmic::iced::Color, cosmic::iced::Color)>,
    pub border_color: Option<cosmic::iced::Color>,
    pub border_width: f32,
    pub border_dashed: bool,
    pub padding: [f32; 4],
    pub border_radius: f32,
    pub letter_spacing: f32,
    pub uppercase: bool,
    pub list_style: Option<String>,
    pub shadow_color: Option<cosmic::iced::Color>,
    pub shadow_offset: (f32, f32),
    pub text_transparent: bool,
    pub background_clip_text: bool,
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

        let properties = parse_declarations(props_str);
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

    let props = parse_declarations(props_str);
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

pub fn parse_declarations(block: &str) -> HashMap<String, String> {
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
    resolve_style_in_parent(tag, classes, None, rules)
}

pub fn resolve_style_in_parent(
    tag: &str,
    classes: &[String],
    parent_tag: Option<&str>,
    rules: &[CssRule],
) -> ResolvedStyle {
    let mut style = ResolvedStyle::default();
    let tag_lower = tag.to_lowercase();

    if !classes.is_empty() {
        eprintln!(
            "[css] Resolving style for tag='{}' classes={:?}",
            tag, classes
        );
    }

    for rule in rules {
        let applies = does_selector_match(&rule.selector, &tag_lower, classes)
            || parent_tag.is_some_and(|parent| {
                does_descendant_selector_match(&rule.selector, parent, &tag_lower, classes)
            });
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

    if style.text_transparent
        && let Some((first, second)) = style.gradient
    {
        style.color = Some(mix_colors(first, second));
    }

    style
}

pub fn resolve_style_with_ancestors(
    tag: &str,
    classes: &[String],
    ancestors: &[(&str, &[String])],
    rules: &[CssRule],
) -> ResolvedStyle {
    let mut style = ResolvedStyle::default();
    for rule in rules {
        if selector_matches_context(&rule.selector, tag, classes, ancestors) {
            for (property, value) in &rule.properties {
                apply_property(&mut style, property, value);
            }
        }
    }
    if style.text_transparent
        && let Some((first, second)) = style.gradient
    {
        style.color = Some(mix_colors(first, second));
    }
    style
}

fn selector_matches_context(
    selector: &str,
    tag: &str,
    classes: &[String],
    ancestors: &[(&str, &[String])],
) -> bool {
    selector.split(',').any(|candidate| {
        let parts: Vec<_> = candidate.split_whitespace().collect();
        let Some(last) = parts.last() else {
            return false;
        };
        if !selector_part_matches(last, tag, classes) {
            return false;
        }
        let mut ancestor_index = ancestors.len();
        for part in parts[..parts.len() - 1].iter().rev() {
            let mut found = false;
            while ancestor_index > 0 {
                ancestor_index -= 1;
                let (ancestor_tag, ancestor_classes) = ancestors[ancestor_index];
                if selector_part_matches(part, ancestor_tag, ancestor_classes) {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    })
}

fn does_descendant_selector_match(
    selector: &str,
    parent_tag: &str,
    tag: &str,
    classes: &[String],
) -> bool {
    selector.split(',').any(|selector| {
        let parts: Vec<_> = selector.split_whitespace().collect();
        parts.len() == 2
            && selector_part_matches(parts[0], &parent_tag.to_lowercase(), &[])
            && selector_part_matches(parts[1], tag, classes)
    })
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
    fn image_rule_matches_inside_heading_context() {
        let (rules, _) = parse_css("h3 img { width: 2em; height: 1.5em; vertical-align: middle; }");
        let style = resolve_style_in_parent("img", &[], Some("h3"), &rules);

        assert_eq!(style.width, Some(28.0));
        assert_eq!(style.height, Some(21.0));
        assert!(style.vertical_middle);
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

    #[test]
    fn resolves_drop_cap_float_and_relative_size() {
        let (rules, _) = parse_css(
            ".capitular { float: left; font-size: 3.2em; margin: -0.1em 0.025em -0.2em 0; }",
        );
        let style = resolve_style("span", &["capitular".to_string()], &rules);

        assert!(style.float_left);
        assert_eq!(style.font_size, Some(44.8));
        assert!((style.margin_right - 0.35).abs() < 0.001);
    }

    #[test]
    fn resolves_table_descendants_even_rows_and_visual_properties() {
        let (rules, _) = parse_css(
            r#"
            .tabla th { background-color: #3498db; color: white; padding: 12px; }
            .tabla tr:nth-child(even) { background-color: #f2f2f2; }
            .tabla td { border: 2px dashed #764ba2; font-family: Verdana; }
            "#,
        );
        let table_classes = vec!["tabla".to_string()];
        let even_classes = vec!["__folio-even".to_string()];
        let no_classes: Vec<String> = Vec::new();

        let heading =
            resolve_style_with_ancestors("th", &no_classes, &[("table", &table_classes)], &rules);
        assert_eq!(heading.padding, [12.0; 4]);
        assert_eq!(heading.color, Some(cosmic::iced::Color::WHITE));

        let row =
            resolve_style_with_ancestors("tr", &even_classes, &[("table", &table_classes)], &rules);
        assert!(row.background_color.is_some());

        let cell =
            resolve_style_with_ancestors("td", &no_classes, &[("table", &table_classes)], &rules);
        assert_eq!(cell.border_width, 2.0);
        assert!(cell.border_dashed);
        assert_eq!(cell.font_family.as_deref(), Some("Verdana"));
    }

    #[test]
    fn resolves_gradient_text_shadow_transform_and_spacing() {
        let (rules, _) = parse_css(
            r#"h4 {
                background: linear-gradient(to right, #8e44ad, #3498db);
                -webkit-background-clip: text;
                -webkit-text-fill-color: transparent;
                text-shadow: 2px 2px 4px rgba(0,0,0,0.2);
                text-transform: uppercase;
                letter-spacing: 2px;
            }"#,
        );
        let style = resolve_style("h4", &[], &rules);
        assert!(style.gradient.is_some());
        assert!(style.color.is_some());
        assert!(style.shadow_color.is_some());
        assert_eq!(style.shadow_offset, (2.0, 2.0));
        assert!(style.uppercase);
        assert_eq!(style.letter_spacing, 2.0);
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
    let mut part = part.trim();
    if let Some(base) = part.strip_suffix(":nth-child(even)") {
        if !classes.iter().any(|class| class == "__folio-even") {
            return false;
        }
        part = base;
    }
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
        "float" => {
            style.float_left = value.eq_ignore_ascii_case("left");
        }
        "margin-right" => {
            style.margin_right = parse_css_length(value).unwrap_or(0.0);
        }
        "margin" => {
            let values: Vec<_> = value.split_whitespace().collect();
            let right = match values.as_slice() {
                [_all] => values[0],
                [_vertical, horizontal] => horizontal,
                [_, right, _] | [_, right, _, _] => right,
                _ => "0",
            };
            style.margin_right = parse_css_length(right).unwrap_or(0.0);
        }
        "width" => {
            if let Some(percent) = value.trim().strip_suffix('%') {
                style.width_percent = percent
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .map(|value| value / 100.0);
            } else {
                style.width = parse_css_length(value);
            }
        }
        "height" => style.height = parse_css_length(value),
        "vertical-align" => {
            style.vertical_middle = value.eq_ignore_ascii_case("middle");
        }
        "background-color" => style.background_color = parse_css_color(value),
        "background" => {
            style.gradient = parse_linear_gradient(value);
            if style.gradient.is_none() {
                style.background_color = parse_css_color(value);
            }
        }
        "border" | "border-left" | "border-bottom" => apply_border(style, value),
        "border-width" => style.border_width = parse_css_length(value).unwrap_or(0.0),
        "border-color" => style.border_color = parse_css_color(value),
        "padding" => style.padding = parse_box_lengths(value),
        "padding-top" => style.padding[0] = parse_css_length(value).unwrap_or(0.0),
        "padding-right" => style.padding[1] = parse_css_length(value).unwrap_or(0.0),
        "padding-bottom" => style.padding[2] = parse_css_length(value).unwrap_or(0.0),
        "padding-left" => style.padding[3] = parse_css_length(value).unwrap_or(0.0),
        "border-radius" => style.border_radius = parse_css_length(value).unwrap_or(0.0),
        "letter-spacing" => style.letter_spacing = parse_css_length(value).unwrap_or(0.0),
        "text-transform" => style.uppercase = value.eq_ignore_ascii_case("uppercase"),
        "list-style-type" => style.list_style = Some(value.trim().to_string()),
        "text-shadow" | "box-shadow" => apply_shadow(style, value),
        "-webkit-text-fill-color" if value.eq_ignore_ascii_case("transparent") => {
            style.text_transparent = true;
        }
        "-webkit-background-clip" | "background-clip" => {
            style.background_clip_text = value.eq_ignore_ascii_case("text");
        }
        _ => {}
    }
}

pub fn inherit_style(parent: &ResolvedStyle, child: &ResolvedStyle) -> ResolvedStyle {
    let mut inherited = child.clone();
    inherited.font_family = child
        .font_family
        .clone()
        .or_else(|| parent.font_family.clone());
    inherited.font_size = child.font_size.or(parent.font_size);
    inherited.color = child.color.or(parent.color);
    inherited.bold |= parent.bold;
    inherited.italic |= parent.italic;
    inherited.underline |= parent.underline;
    inherited.letter_spacing = if child.letter_spacing != 0.0 {
        child.letter_spacing
    } else {
        parent.letter_spacing
    };
    inherited.uppercase |= parent.uppercase;
    inherited
}

fn apply_border(style: &mut ResolvedStyle, value: &str) {
    for token in value.split_whitespace() {
        if let Some(width) = parse_css_length(token) {
            style.border_width = width;
        } else if matches!(token, "dashed" | "dotted") {
            style.border_dashed = true;
        } else if let Some(color) = parse_css_color(token) {
            style.border_color = Some(color);
        }
    }
}

fn parse_box_lengths(value: &str) -> [f32; 4] {
    let values: Vec<f32> = value
        .split_whitespace()
        .filter_map(parse_css_length)
        .collect();
    match values.as_slice() {
        [all] => [*all; 4],
        [vertical, horizontal] => [*vertical, *horizontal, *vertical, *horizontal],
        [top, horizontal, bottom] => [*top, *horizontal, *bottom, *horizontal],
        [top, right, bottom, left, ..] => [*top, *right, *bottom, *left],
        _ => [0.0; 4],
    }
}

fn apply_shadow(style: &mut ResolvedStyle, value: &str) {
    let mut offsets = value.split_whitespace().filter_map(parse_css_length);
    style.shadow_offset = (offsets.next().unwrap_or(0.0), offsets.next().unwrap_or(0.0));
    style.shadow_color = value.split_whitespace().find_map(parse_css_color);
}

fn parse_linear_gradient(value: &str) -> Option<(cosmic::iced::Color, cosmic::iced::Color)> {
    if !value.contains("linear-gradient") {
        return None;
    }
    let colors: Vec<_> = value
        .split(|character: char| character == ',' || character == '(' || character == ')')
        .filter_map(|part| part.split_whitespace().find_map(parse_css_color))
        .collect();
    Some((*colors.first()?, *colors.get(1)?))
}

fn mix_colors(a: cosmic::iced::Color, b: cosmic::iced::Color) -> cosmic::iced::Color {
    cosmic::iced::Color::from_rgba(
        (a.r + b.r) / 2.0,
        (a.g + b.g) / 2.0,
        (a.b + b.b) / 2.0,
        (a.a + b.a) / 2.0,
    )
}

fn parse_font_size(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.ends_with("pt") {
        value[..value.len() - 2].trim().parse().ok()
    } else if value.ends_with("px") {
        let px: f32 = value[..value.len() - 2].trim().parse().ok()?;
        Some(px * 0.75)
    } else if value.ends_with("rem") {
        let rem: f32 = value[..value.len() - 3].trim().parse().ok()?;
        Some(rem * 14.0)
    } else if value.ends_with("em") {
        let em: f32 = value[..value.len() - 2].trim().parse().ok()?;
        Some(em * 14.0)
    } else {
        value.parse().ok()
    }
}

fn parse_css_length(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(value) = value.strip_suffix("em") {
        return Some(value.trim().parse::<f32>().ok()? * 14.0);
    }
    if let Some(value) = value.strip_suffix("pt") {
        return value.trim().parse().ok();
    }
    if let Some(value) = value.strip_suffix("px") {
        return Some(value.trim().parse::<f32>().ok()? * 0.75);
    }
    value.parse().ok()
}

fn parse_css_color(value: &str) -> Option<cosmic::iced::Color> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "white" => return Some(cosmic::iced::Color::WHITE),
        "black" => return Some(cosmic::iced::Color::BLACK),
        "transparent" => return Some(cosmic::iced::Color::TRANSPARENT),
        "red" => return Some(cosmic::iced::Color::from_rgb8(255, 0, 0)),
        "blue" => return Some(cosmic::iced::Color::from_rgb8(0, 0, 255)),
        "green" => return Some(cosmic::iced::Color::from_rgb8(0, 128, 0)),
        _ => {}
    }
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
    if let Some(inner) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let parts: Vec<_> = inner.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return Some(cosmic::iced::Color::from_rgba8(
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
                parts[3].parse::<f32>().ok()?,
            ));
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
