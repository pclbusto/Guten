use std::cell::RefCell;
use std::rc::Rc;

use cosmic::iced::advanced::graphics::text;
use cosmic::iced::advanced::image::Handle as ImageHandle;
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::font::{Family, Style as FontStyle, Weight};
use cosmic::iced::mouse;
use cosmic::iced::widget::canvas::{self, Cache, Frame, Geometry, Stroke};
use cosmic::iced::{Color, Font, Point, Rectangle, Size};
use cosmic_text::{Buffer, FontSystem, Metrics, Shaping};

use crate::content::{BlockStyle, ContentBlock, StyleMap, StyledSpan};
use crate::css::{self, CssRule};
use crate::fonts::FontNameMap;
use crate::image_resources::{ImageMetadataCache, placeholder_label, scaled_image_size};

const PAGE_MARGIN: f32 = 60.0;
const MISSING_IMAGE_HEIGHT: f32 = 160.0;
const MOUSE_WHEEL_STEP: f32 = 120.0;
const DEFAULT_BASE_SIZE_PT: f32 = 14.0;

#[derive(Default, Debug)]
pub struct ReaderMetrics {
    pub scroll_y: f32,
    pub viewport_h: f32,
    pub total_h: f32,
}

pub struct TextCanvas<M> {
    blocks: Vec<ContentBlock>,
    style_map: StyleMap,
    css_rules: Vec<CssRule>,
    font_name_map: FontNameMap,
    bg_color: Color,
    base_color: Color,
    base_font_family: Family,
    text_scale: f32,
    scroll_y: f32,
    cache: Cache,
    reader_metrics: Rc<RefCell<ReaderMetrics>>,
    image_metadata_cache: Rc<RefCell<ImageMetadataCache>>,
    layout_cache: Rc<RefCell<ReaderLayoutCache>>,
    on_scroll: Box<dyn Fn(f32) -> M>,
    on_image_click: Box<dyn Fn(String) -> M>,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedRunStyle {
    font_size: f32,
    line_height: f32,
    color: Color,
    font: Font,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedBlockStyle {
    run: ResolvedRunStyle,
    bold: bool,
    italic: bool,
    align: Horizontal,
}

#[derive(Debug, Clone)]
struct StyledRun {
    text: String,
    style: ResolvedRunStyle,
}

#[derive(Debug, Clone)]
struct LayoutRun {
    text: String,
    style: ResolvedRunStyle,
    x: f32,
    width: f32,
}

#[derive(Debug, Clone)]
struct LayoutLine {
    runs: Vec<LayoutRun>,
    y: f32,
    height: f32,
    baseline: f32,
    width: f32,
    align: Horizontal,
}

#[derive(Debug, Clone)]
enum LayoutBlock {
    Paragraph {
        lines: Vec<LayoutLine>,
        rect: Rectangle,
    },
    Heading {
        lines: Vec<LayoutLine>,
        rect: Rectangle,
        level: u8,
    },
    Image {
        rect: Rectangle,
        src: String,
        handle: ImageHandle,
        alt: Option<String>,
    },
    ImagePlaceholder {
        rect: Rectangle,
        label: String,
    },
    Separator {
        rect: Rectangle,
    },
}

pub struct ReaderLayoutCache {
    layout: Vec<LayoutBlock>,
    viewport: Size,
    total_h: f32,
    dirty: bool,
}

impl Default for ReaderLayoutCache {
    fn default() -> Self {
        Self {
            layout: Vec::new(),
            viewport: Size::ZERO,
            total_h: 0.0,
            dirty: true,
        }
    }
}

impl ReaderLayoutCache {
    pub fn clear(&mut self) {
        self.layout.clear();
        self.total_h = 0.0;
        self.dirty = true;
    }
}

impl LayoutBlock {
    fn rect(&self) -> Rectangle {
        match self {
            Self::Paragraph { rect, .. }
            | Self::Heading { rect, .. }
            | Self::Image { rect, .. }
            | Self::ImagePlaceholder { rect, .. }
            | Self::Separator { rect } => *rect,
        }
    }
}

#[derive(Default)]
pub struct ScrollDrag {
    dragging: bool,
    drag_start_y: f32,
    drag_start_scroll: f32,
}

impl<M: Clone + 'static> TextCanvas<M> {
    pub fn new(
        blocks: &[ContentBlock],
        style_map: &StyleMap,
        css_rules: &[CssRule],
        base_size: f32,
        bg_color: Color,
        base_color: Color,
        base_font_family: Family,
        font_name_map: &FontNameMap,
        scroll_y: f32,
        reader_metrics: Rc<RefCell<ReaderMetrics>>,
        image_metadata_cache: Rc<RefCell<ImageMetadataCache>>,
        layout_cache: Rc<RefCell<ReaderLayoutCache>>,
        on_scroll: impl Fn(f32) -> M + 'static,
        on_image_click: impl Fn(String) -> M + 'static,
    ) -> Self {
        Self {
            blocks: blocks.to_vec(),
            style_map: style_map.clone(),
            css_rules: css_rules.to_vec(),
            font_name_map: font_name_map.clone(),
            bg_color,
            base_color,
            base_font_family,
            text_scale: base_size / DEFAULT_BASE_SIZE_PT,
            scroll_y,
            cache: Cache::new(),
            reader_metrics,
            image_metadata_cache,
            layout_cache,
            on_scroll: Box::new(on_scroll),
            on_image_click: Box::new(on_image_click),
        }
    }

    fn layout_blocks(&self, viewport: Size) {
        let content_w = (viewport.width - PAGE_MARGIN * 2.0).max(100.0);
        let mut blocks = Vec::new();
        let mut y = PAGE_MARGIN;
        let mut font_system = match text::font_system().write() {
            Ok(font_system) => font_system,
            Err(poisoned) => poisoned.into_inner(),
        };

        for block in &self.blocks {
            match block {
                ContentBlock::Heading {
                    level,
                    spans,
                    classes,
                } => {
                    let tag = format!("h{level}");
                    let css_style = css::resolve_style(&tag, classes, &self.css_rules);
                    let resolved =
                        self.resolve_block_style(&css_style, self.style_map.heading_style(*level));
                    let (lines, rect) =
                        self.layout_text_block(spans, resolved, content_w, y, font_system.raw());
                    blocks.push(LayoutBlock::Heading {
                        lines,
                        rect,
                        level: *level,
                    });
                    y += rect.height + 8.0;
                }
                ContentBlock::Paragraph { spans, classes } => {
                    let css_style = css::resolve_style("p", classes, &self.css_rules);
                    let resolved = self.resolve_block_style(&css_style, &self.style_map.p);
                    let (lines, rect) =
                        self.layout_text_block(spans, resolved, content_w, y, font_system.raw());
                    blocks.push(LayoutBlock::Paragraph { lines, rect });
                    y += rect.height + 4.0;
                }
                ContentBlock::Image { src, alt } => {
                    let dimensions = self.image_metadata_cache.borrow_mut().dimensions(src);
                    if let Some((width, height)) = dimensions.and_then(|(width, height)| {
                        scaled_image_size(
                            width,
                            height,
                            content_w,
                            (viewport.height * 0.75).max(1.0),
                        )
                    }) {
                        let rect = Rectangle::new(
                            Point::new(PAGE_MARGIN + (content_w - width) / 2.0, y),
                            Size::new(width, height),
                        );
                        blocks.push(LayoutBlock::Image {
                            rect,
                            src: src.clone(),
                            handle: ImageHandle::from_path(src),
                            alt: (!alt.is_empty()).then(|| alt.clone()),
                        });
                        y += height + 8.0;
                    } else {
                        let rect = Rectangle::new(
                            Point::new(PAGE_MARGIN, y),
                            Size::new(content_w, MISSING_IMAGE_HEIGHT),
                        );
                        blocks.push(LayoutBlock::ImagePlaceholder {
                            rect,
                            label: placeholder_label(src, alt),
                        });
                        y += MISSING_IMAGE_HEIGHT + 8.0;
                    }
                }
                ContentBlock::Separator => {
                    let rect =
                        Rectangle::new(Point::new(PAGE_MARGIN, y), Size::new(content_w, 12.0));
                    blocks.push(LayoutBlock::Separator { rect });
                    y += rect.height;
                }
            }
        }

        let mut cache = self.layout_cache.borrow_mut();
        cache.layout = blocks;
        cache.viewport = viewport;
        cache.total_h = y + PAGE_MARGIN;
        cache.dirty = false;
    }

    fn resolve_block_style(
        &self,
        css_style: &css::ResolvedStyle,
        block_style: &BlockStyle,
    ) -> ResolvedBlockStyle {
        let font_size = css_style
            .font_size
            .map(|size| size * self.text_scale)
            .unwrap_or(block_style.font_size);
        let bold = css_style.bold || block_style.bold;
        let italic = css_style.italic || block_style.italic;

        ResolvedBlockStyle {
            run: ResolvedRunStyle {
                font_size,
                line_height: font_size * block_style.line_height.unwrap_or(1.3),
                color: css_style.color.unwrap_or(self.base_color),
                font: font_for(
                    css_style,
                    self.base_font_family,
                    bold,
                    italic,
                    &self.font_name_map,
                ),
            },
            bold,
            italic,
            align: css_style
                .align
                .or(block_style.align)
                .unwrap_or(Horizontal::Left),
        }
    }

    fn layout_text_block(
        &self,
        spans: &[StyledSpan],
        resolved: ResolvedBlockStyle,
        content_w: f32,
        y: f32,
        font_system: &mut FontSystem,
    ) -> (Vec<LayoutLine>, Rectangle) {
        let runs = styled_runs(
            spans,
            resolved.run,
            resolved.bold,
            resolved.italic,
            &self.css_rules,
            &self.font_name_map,
            self.text_scale,
        );
        let (lines, height) =
            layout_paragraph(runs, content_w, PAGE_MARGIN, y, resolved.align, font_system);
        let rect = Rectangle::new(Point::new(PAGE_MARGIN, y), Size::new(content_w, height));
        (lines, rect)
    }

    fn clamp_scroll(&self, value: f32) -> f32 {
        let cache = self.layout_cache.borrow();
        let max = (cache.total_h - cache.viewport.height).max(0.0);
        value.clamp(0.0, max)
    }

    fn scrolled_message(&self, scroll_y: f32) -> M {
        (self.on_scroll)(self.clamp_scroll(scroll_y))
    }

    fn image_clicked_message(&self, src: String) -> M {
        (self.on_image_click)(src)
    }

    fn scrollbar_metrics(&self, viewport_h: f32) -> Option<(f32, f32, f32)> {
        let total_h = self.layout_cache.borrow().total_h;
        if total_h <= viewport_h {
            return None;
        }
        let max_scroll = (total_h - viewport_h).max(1.0);
        let bar_h = (viewport_h * viewport_h / total_h).max(20.0);
        let bar_y = self.scroll_y / max_scroll * (viewport_h - bar_h);
        Some((bar_h, max_scroll, bar_y))
    }

    fn needs_layout(&self, viewport: Size) -> bool {
        let cache = self.layout_cache.borrow();
        cache.dirty
            || (cache.viewport.width - viewport.width).abs() > 1.0
            || (cache.viewport.height - viewport.height).abs() > 1.0
    }

    fn thumb_hit_rect(&self, bounds: Rectangle) -> Option<Rectangle> {
        let (bar_h, _, bar_y) = self.scrollbar_metrics(bounds.height)?;
        Some(Rectangle::new(
            Point::new(bounds.width - 8.0, bar_y),
            Size::new(8.0, bar_h),
        ))
    }

    fn track_hit_rect(&self, bounds: Rectangle) -> Option<Rectangle> {
        self.scrollbar_metrics(bounds.height)?;
        Some(Rectangle::new(
            Point::new(bounds.width - 8.0, 0.0),
            Size::new(8.0, bounds.height),
        ))
    }

    fn image_at(&self, bounds: Rectangle, pos: Point) -> Option<String> {
        if pos.x < 0.0 || pos.y < 0.0 || pos.x > bounds.width || pos.y > bounds.height {
            return None;
        }

        let document_pos = Point::new(pos.x, pos.y + self.scroll_y);
        self.layout_cache
            .borrow()
            .layout
            .iter()
            .find_map(|block| match block {
                LayoutBlock::Image { rect, src, .. } if rect.contains(document_pos) => {
                    Some(src.clone())
                }
                _ => None,
            })
    }
}

fn font_for(
    css: &css::ResolvedStyle,
    base: Family,
    bold: bool,
    italic: bool,
    font_name_map: &FontNameMap,
) -> Font {
    let (family, resolved_bold, resolved_italic) = css
        .font_family
        .as_ref()
        .and_then(|family| font_name_map.resolve(family, bold, italic))
        .unwrap_or((base, bold, italic));
    Font {
        family,
        weight: if resolved_bold {
            Weight::Bold
        } else {
            Weight::Normal
        },
        style: if resolved_italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..Default::default()
    }
}

fn styled_runs(
    spans: &[StyledSpan],
    base: ResolvedRunStyle,
    base_bold: bool,
    base_italic: bool,
    css_rules: &[CssRule],
    font_name_map: &FontNameMap,
    text_scale: f32,
) -> Vec<StyledRun> {
    spans
        .iter()
        .filter(|span| !span.text.is_empty())
        .map(|span| {
            let span_css = if span.classes.is_empty() {
                css::ResolvedStyle::default()
            } else {
                css::resolve_style("span", &span.classes, css_rules)
            };
            let font_size = span_css
                .font_size
                .or(span.size)
                .map(|size| size * text_scale)
                .unwrap_or(base.font_size);
            let bold = span_css.bold || span.bold || base_bold;
            let italic = span_css.italic || span.italic || base_italic;
            let font = if span_css.font_family.is_some() {
                font_for(&span_css, base.font.family, bold, italic, font_name_map)
            } else if span.bold || span.italic {
                Font {
                    family: base.font.family,
                    weight: if bold { Weight::Bold } else { Weight::Normal },
                    style: if italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    },
                    ..Default::default()
                }
            } else {
                base.font
            };

            StyledRun {
                text: span.text.clone(),
                style: ResolvedRunStyle {
                    font_size,
                    line_height: base.line_height.max(font_size * 1.3),
                    color: span_css.color.or(span.color).unwrap_or(base.color),
                    font,
                },
            }
        })
        .collect()
}

#[derive(Default)]
struct LineBuilder {
    runs: Vec<LayoutRun>,
    width: f32,
    height: f32,
    baseline: f32,
}

impl LineBuilder {
    fn push(&mut self, text: String, style: ResolvedRunStyle, width: f32) {
        self.runs.push(LayoutRun {
            text,
            style,
            x: self.width,
            width,
        });
        self.width += width;
        self.height = self.height.max(style.line_height);
        // TODO: derive ascent/descent from shaped glyph metrics for mixed-size runs.
        self.baseline = self.baseline.max(style.font_size);
    }

    fn finish(&mut self, y: f32, align: Horizontal) -> Option<LayoutLine> {
        if self.runs.is_empty() {
            return None;
        }
        Some(LayoutLine {
            runs: std::mem::take(&mut self.runs),
            y,
            height: self.height,
            baseline: self.baseline,
            width: self.width,
            align,
        })
    }
}

fn layout_paragraph(
    runs: Vec<StyledRun>,
    max_width: f32,
    _x: f32,
    y: f32,
    align: Horizontal,
    font_system: &mut FontSystem,
) -> (Vec<LayoutLine>, f32) {
    let mut lines = Vec::new();
    let mut line = LineBuilder::default();
    let mut line_y = y;
    let mut pending_space: Option<(String, ResolvedRunStyle, f32)> = None;

    for run in runs {
        for token in split_preserving_spaces(&run.text) {
            let width = measure_text(token, run.style, font_system);
            if token.chars().all(char::is_whitespace) {
                if !line.runs.is_empty() {
                    pending_space = Some((token.to_string(), run.style, width));
                }
                continue;
            }

            let space_width = pending_space.as_ref().map_or(0.0, |(_, _, width)| *width);
            if !line.runs.is_empty() && line.width + space_width + width > max_width {
                let height = line.height;
                if let Some(finished) = line.finish(line_y, align) {
                    lines.push(finished);
                    line_y += height;
                }
                line = LineBuilder::default();
                pending_space = None;
            }

            if !line.runs.is_empty() {
                if let Some((space, style, width)) = pending_space.take() {
                    line.push(space, style, width);
                }
            } else {
                pending_space = None;
            }
            line.push(token.to_string(), run.style, width);
        }
    }

    if let Some(finished) = line.finish(line_y, align) {
        line_y += finished.height;
        lines.push(finished);
    }

    (lines, line_y - y)
}

fn split_preserving_spaces(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut whitespace = None;

    for (index, ch) in text.char_indices() {
        let is_whitespace = ch.is_whitespace();
        match whitespace {
            None => whitespace = Some(is_whitespace),
            Some(previous) if previous != is_whitespace => {
                tokens.push(&text[start..index]);
                start = index;
                whitespace = Some(is_whitespace);
            }
            _ => {}
        }
    }
    if start < text.len() {
        tokens.push(&text[start..]);
    }
    tokens
}

fn measure_text(text_value: &str, style: ResolvedRunStyle, font_system: &mut FontSystem) -> f32 {
    if text_value.is_empty() {
        return 0.0;
    }
    let mut buffer = Buffer::new(
        font_system,
        Metrics::new(style.font_size, style.line_height),
    );
    buffer.set_size(None, None);
    buffer.set_text(
        text_value,
        &text::to_attributes(style.font),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(font_system, false);
    buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max)
}

fn line_x(line: &LayoutLine, rect: Rectangle) -> f32 {
    match line.align {
        Horizontal::Left => rect.x,
        Horizontal::Center => rect.x + (rect.width - line.width) / 2.0,
        Horizontal::Right => rect.x + rect.width - line.width,
    }
}

impl<M: Clone + 'static> canvas::Program<M, cosmic::Theme, cosmic::iced::Renderer> for TextCanvas<M> {
    type State = ScrollDrag;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::iced::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<cosmic::iced::Renderer>> {
        if self.needs_layout(bounds.size()) {
            self.layout_blocks(bounds.size());
        }

        let geometry = self
            .cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.bg_color);
                let scroll = self.scroll_y;

                let layout_cache = self.layout_cache.borrow();
                for block in &layout_cache.layout {
                    let rect = block.rect();
                    let visible_y = rect.y - scroll;
                    if visible_y > bounds.height || visible_y + rect.height < 0.0 {
                        continue;
                    }

                    match block {
                        LayoutBlock::Paragraph { lines, rect }
                        | LayoutBlock::Heading { lines, rect, .. } => {
                            for line in lines {
                                let origin_x = line_x(line, *rect);
                                for run in &line.runs {
                                    let run_y =
                                        line.y - scroll + line.baseline - run.style.font_size;
                                    frame.fill_text(canvas::Text {
                                        content: run.text.clone(),
                                        position: Point::new(origin_x + run.x, run_y),
                                        max_width: run.width.max(1.0),
                                        color: run.style.color,
                                        size: run.style.font_size.into(),
                                        line_height: cosmic::iced::Pixels(run.style.line_height)
                                            .into(),
                                        font: run.style.font,
                                        align_x: cosmic::iced::alignment::Horizontal::Left.into(),
                                        align_y: cosmic::iced::alignment::Vertical::Top.into(),
                                        shaping: cosmic::iced::widget::text::Shaping::Advanced,
                                    });
                                }
                            }
                        }
                        LayoutBlock::Image {
                            rect, handle, alt, ..
                        } => {
                            let _accessible_description = alt.as_deref();
                            frame.draw_image(
                                Rectangle::new(Point::new(rect.x, rect.y - scroll), rect.size()),
                                handle,
                            );
                        }
                        LayoutBlock::ImagePlaceholder { rect, label } => {
                            let top_left = Point::new(rect.x, rect.y - scroll);
                            frame.fill_rectangle(
                                top_left,
                                rect.size(),
                                Color::from_rgba8(128, 128, 128, 0.08),
                            );
                            frame.stroke_rectangle(
                                top_left,
                                rect.size(),
                                Stroke::default()
                                    .with_color(Color::from_rgba8(128, 128, 128, 0.45))
                                    .with_width(1.0),
                            );
                            frame.fill_text(canvas::Text {
                                content: label.clone(),
                                position: Point::new(
                                    rect.center_x(),
                                    rect.y - scroll + rect.height / 2.0,
                                ),
                                max_width: rect.width - 24.0,
                                color: Color::from_rgba8(90, 90, 90, 0.9),
                                size: (13.0 * self.text_scale).into(),
                                align_x: cosmic::iced::alignment::Horizontal::Center.into(),
                                align_y: cosmic::iced::alignment::Vertical::Center.into(),
                                ..canvas::Text::default()
                            });
                        }
                        LayoutBlock::Separator { .. } => {}
                    }
                }

                if let Some((bar_h, _, bar_y)) = self.scrollbar_metrics(bounds.height) {
                    frame.fill_rectangle(
                        Point::new(bounds.width - 8.0, bar_y),
                        Size::new(4.0, bar_h),
                        Color::from_rgba8(128, 128, 128, 0.5),
                    );
                }
            });

        let mut metrics = self.reader_metrics.borrow_mut();
        metrics.scroll_y = self.scroll_y;
        metrics.viewport_h = bounds.height;
        metrics.total_h = self.layout_cache.borrow().total_h;

        vec![geometry]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<M>> {
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * MOUSE_WHEEL_STEP,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                return Some(canvas::Action::publish(
                    self.scrolled_message(self.scroll_y - dy),
                ));
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let pos = cursor.position_in(bounds)?;

                if let Some(thumb) = self.thumb_hit_rect(bounds) {
                    if thumb.contains(pos) {
                        state.dragging = true;
                        state.drag_start_y = pos.y;
                        state.drag_start_scroll = self.scroll_y;
                        return Some(canvas::Action::capture());
                    }
                }

                if let Some(track) = self.track_hit_rect(bounds) {
                    if track.contains(pos) {
                        let (bar_h, max_scroll, _) = self.scrollbar_metrics(bounds.height)?;
                        let new_scroll =
                            ((pos.y - bar_h / 2.0) / (bounds.height - bar_h)) * max_scroll;
                        return Some(canvas::Action::publish(self.scrolled_message(new_scroll)));
                    }
                }

                if let Some(src) = self.image_at(bounds, pos) {
                    return Some(canvas::Action::publish(self.image_clicked_message(src)));
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !state.dragging {
                    return None;
                }
                let global_pos = cursor.position()?;
                let local_y = global_pos.y - bounds.y;
                let (bar_h, max_scroll, _) = self.scrollbar_metrics(bounds.height)?;
                let dy = local_y - state.drag_start_y;
                let new_scroll =
                    state.drag_start_scroll + dy / (bounds.height - bar_h) * max_scroll;
                return Some(canvas::Action::publish(self.scrolled_message(new_scroll)));
            }
            _ => {}
        }
        None
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging {
            return mouse::Interaction::Grabbing;
        }

        if let Some(pos) = cursor.position_in(bounds) {
            if self
                .track_hit_rect(bounds)
                .is_some_and(|track| track.contains(pos))
            {
                return mouse::Interaction::Grab;
            }
            if self.image_at(bounds, pos).is_some() {
                return mouse::Interaction::Pointer;
            }
        }

        mouse::Interaction::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    enum TestMessage {
        Scroll(f32),
        Image(String),
    }

    fn dummy_canvas(
        blocks: &[ContentBlock],
        scroll_y: f32,
        layout_cache: Rc<RefCell<ReaderLayoutCache>>,
    ) -> TextCanvas<TestMessage> {
        TextCanvas::new(
            blocks,
            &StyleMap::default(),
            &[],
            DEFAULT_BASE_SIZE_PT,
            Color::WHITE,
            Color::BLACK,
            Family::SansSerif,
            &FontNameMap::default(),
            scroll_y,
            std::rc::Rc::new(std::cell::RefCell::new(ReaderMetrics::default())),
            std::rc::Rc::new(std::cell::RefCell::new(ImageMetadataCache::default())),
            layout_cache,
            |y| TestMessage::Scroll(y),
            |src| TestMessage::Image(src),
        )
    }

    fn normal_style() -> ResolvedRunStyle {
        ResolvedRunStyle {
            font_size: 14.0,
            line_height: 18.2,
            color: Color::BLACK,
            font: Font::DEFAULT,
        }
    }

    fn layout_test_paragraph(text_value: &str, max_width: f32) -> Vec<LayoutLine> {
        let runs = vec![StyledRun {
            text: text_value.into(),
            style: normal_style(),
        }];
        let mut guard = text::font_system()
            .write()
            .unwrap_or_else(|error| error.into_inner());
        layout_paragraph(runs, max_width, 0.0, 0.0, Horizontal::Left, guard.raw()).0
    }

    #[test]
    fn split_preserves_word_and_space_boundaries() {
        assert_eq!(
            split_preserving_spaces("Hola  mundo, cómo va."),
            vec!["Hola", "  ", "mundo,", " ", "cómo", " ", "va."]
        );
    }

    #[test]
    fn mixed_runs_share_a_layout_line() {
        let normal = normal_style();
        let bold = ResolvedRunStyle {
            font: Font {
                weight: Weight::Bold,
                ..Font::DEFAULT
            },
            ..normal
        };
        let runs = vec![
            StyledRun {
                text: "Hola ".into(),
                style: normal,
            },
            StyledRun {
                text: "mundo".into(),
                style: bold,
            },
            StyledRun {
                text: ", cómo va.".into(),
                style: normal,
            },
        ];
        let mut guard = text::font_system()
            .write()
            .unwrap_or_else(|error| error.into_inner());
        let (lines, height) =
            layout_paragraph(runs, 500.0, 0.0, 0.0, Horizontal::Left, guard.raw());

        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0]
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "Hola mundo, cómo va."
        );
        assert!(
            lines[0]
                .runs
                .windows(2)
                .all(|pair| pair[0].x + pair[0].width <= pair[1].x + 0.01)
        );
        assert!(height > 0.0);
    }

    #[test]
    fn text_scale_applies_to_inline_sizes() {
        let base = normal_style();
        let span = StyledSpan {
            text: "Texto".into(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            size: Some(20.0),
            link: None,
            classes: vec![],
        };

        let runs = styled_runs(
            &[span],
            base,
            false,
            false,
            &[],
            &FontNameMap::default(),
            1.5,
        );

        assert_eq!(runs[0].style.font_size, 30.0);
        assert_eq!(runs[0].style.line_height, 39.0);
    }

    #[test]
    fn wraps_long_paragraph_into_multiple_lines() {
        let lines = layout_test_paragraph(
            "Este párrafo contiene suficientes palabras para ocupar varias líneas.",
            90.0,
        );

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.width > 0.0));
    }

    #[test]
    fn does_not_start_line_with_space() {
        let lines = layout_test_paragraph(
            "Primera palabra segunda palabra tercera palabra cuarta palabra",
            85.0,
        );

        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| {
            line.runs
                .first()
                .is_some_and(|run| !run.text.starts_with(char::is_whitespace))
        }));
    }

    #[test]
    fn missing_image_uses_placeholder() {
        let canvas = dummy_canvas(
            &[ContentBlock::Image {
                src: "cover.jpg".into(),
                alt: "Portada".into(),
            }],
            0.0,
            std::rc::Rc::new(std::cell::RefCell::new(ReaderLayoutCache::default())),
        );

        canvas.layout_blocks(Size::new(600.0, 800.0));
        let layout_cache = canvas.layout_cache.borrow();
        let LayoutBlock::ImagePlaceholder { rect, label } = &layout_cache.layout[0] else {
            panic!("expected image placeholder");
        };
        assert_eq!(rect.height, MISSING_IMAGE_HEIGHT);
        assert_eq!(label, "Imagen no disponible: Portada");
        drop(layout_cache);

        assert!(!canvas.needs_layout(Size::new(600.0, 800.0)));
        assert!(canvas.needs_layout(Size::new(700.0, 800.0)));
        canvas.layout_cache.borrow_mut().clear();
        assert!(canvas.needs_layout(Size::new(600.0, 800.0)));
    }

    #[test]
    fn image_hit_test_accounts_for_scroll() {
        let layout_cache = std::rc::Rc::new(std::cell::RefCell::new(ReaderLayoutCache {
            layout: vec![LayoutBlock::Image {
                rect: Rectangle::new(Point::new(60.0, 200.0), Size::new(240.0, 160.0)),
                src: "/book/Images/photo.jpg".into(),
                handle: ImageHandle::from_path("/book/Images/photo.jpg"),
                alt: Some("Foto".into()),
            }],
            viewport: Size::new(600.0, 800.0),
            total_h: 1000.0,
            dirty: false,
        }));
        let canvas = dummy_canvas(&[], 120.0, layout_cache);
        let bounds = Rectangle::new(Point::ORIGIN, Size::new(600.0, 800.0));

        assert_eq!(
            canvas.image_at(bounds, Point::new(100.0, 100.0)),
            Some("/book/Images/photo.jpg".into())
        );
        assert_eq!(canvas.image_at(bounds, Point::new(400.0, 100.0)), None);
    }
}
