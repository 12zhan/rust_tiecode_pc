use gpui::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tiecode::sweetline::{Engine, Document, DocumentAnalyzer, HighlightSpan};
use crate::editor::grammar::*;

pub struct MarkdownViewer {
    content: String,
    blocks: Vec<Block>,
    sweetline_engine: Engine,
    style_cache: HashMap<u32, Hsla>,
    focus_handle: FocusHandle,
    scroll_offset_y: Pixels,
    content_height: Pixels,
}

#[derive(Clone)]
enum Block {
    Heading(usize, String),
    Paragraph(String),
    CodeBlock(Option<String>, Vec<String>, Vec<HighlightSpan>),
    Hr,
}

impl MarkdownViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let engine = Engine::new(true);
        let _ = engine.compile_json(RUST_GRAMMAR);
        let _ = engine.compile_json(CPP_GRAMMAR);
        let _ = engine.compile_json(JSON_GRAMMAR);
        let _ = engine.compile_json(TOML_GRAMMAR);
        let _ = engine.compile_json(YAML_GRAMMAR);
        let _ = engine.compile_json(PYTHON_GRAMMAR);
        let _ = engine.compile_json(JAVASCRIPT_GRAMMAR);
        let _ = engine.compile_json(TYPESCRIPT_GRAMMAR);
        let _ = engine.compile_json(HTML_GRAMMAR);
        let _ = engine.compile_json(CSS_GRAMMAR);
        let _ = engine.compile_json(SHELL_GRAMMAR);
        Self {
            content: String::new(),
            blocks: Vec::new(),
            sweetline_engine: engine,
            style_cache: HashMap::new(),
            focus_handle: cx.focus_handle(),
            scroll_offset_y: px(0.0),
            content_height: px(0.0),
        }
    }

    pub fn set_content(&mut self, text: String, _cx: &mut Context<Self>) {
        self.content = text;
        self.blocks = Self::parse_blocks(&self.content);
        self.prepare_code_blocks();
    }

    fn parse_blocks(text: &str) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut lines = text.lines().peekable();
        while let Some(line) = lines.next() {
            if line.starts_with("```") {
                let lang = line.trim_start_matches("```").trim();
                let lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
                let mut code = Vec::new();
                while let Some(l) = lines.peek() {
                    if l.starts_with("```") {
                        lines.next();
                        break;
                    }
                    code.push(lines.next().unwrap_or_default().to_string());
                }
                blocks.push(Block::CodeBlock(lang, code, Vec::new()));
            } else if line.trim().starts_with("---") || line.trim() == "***" {
                blocks.push(Block::Hr);
            } else if let Some(h) = Self::heading(line) {
                blocks.push(h);
            } else {
                let mut para = String::from(line);
                while let Some(p) = lines.peek() {
                    if p.trim().is_empty() { break; }
                    if p.starts_with("#") || p.starts_with("```") { break; }
                    para.push('\n');
                    para.push_str(lines.next().unwrap_or_default());
                }
                blocks.push(Block::Paragraph(para));
            }
        }
        blocks
    }

    fn heading(line: &str) -> Option<Block> {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if level > 0 {
            let text = trimmed[level..].trim_start().to_string();
            Some(Block::Heading(level.min(6), text))
        } else {
            None
        }
    }

    fn prepare_code_blocks(&mut self) {
        for b in &mut self.blocks {
            if let Block::CodeBlock(lang, lines, spans) = b {
                let uri = Self::uri_for_lang(lang.as_deref());
                let text = lines.join("\n");
                let doc = Document::new(&uri, &text);
                let analyzer = self.sweetline_engine.load_document(&doc);
                let result = analyzer.analyze();
                let parsed = DocumentAnalyzer::parse_result(&result, false);
                *spans = parsed;
                for span in spans.iter() {
                    if !self.style_cache.contains_key(&span.style_id) {
                        let name_opt: Option<String> = self.sweetline_engine.get_style_name(span.style_id);
                        if let Some(name) = name_opt {
                            if let Some(color) = Self::color_for_style(&name) {
                                self.style_cache.insert(span.style_id, color);
                            }
                        }
                    }
                }
            }
        }
    }

    fn uri_for_lang(lang: Option<&str>) -> String {
        match lang.map(|s| s.to_ascii_lowercase()) {
            Some(l) if l == "rs" || l == "rust" => "file:///block.rs".to_string(),
            Some(l) if l == "cpp" || l == "c++" || l == "cc" || l == "hpp" => "file:///block.cpp".to_string(),
            Some(l) if l == "c" || l == "h" => "file:///block.c".to_string(),
            Some(l) if l == "js" || l == "javascript" => "file:///block.js".to_string(),
            Some(l) if l == "ts" || l == "tsx" || l == "typescript" => "file:///block.ts".to_string(),
            Some(l) if l == "py" || l == "python" => "file:///block.py".to_string(),
            Some(l) if l == "json" => "file:///block.json".to_string(),
            Some(l) if l == "toml" => "file:///block.toml".to_string(),
            Some(l) if l == "yaml" || l == "yml" => "file:///block.yaml".to_string(),
            Some(l) if l == "html" => "file:///block.html".to_string(),
            Some(l) if l == "css" => "file:///block.css".to_string(),
            Some(l) if l == "java" => "file:///block.java".to_string(),
            Some(l) if l == "sh" || l == "bash" || l == "zsh" => "file:///block.sh".to_string(),
            _ => "file:///block.txt".to_string(),
        }
    }

    fn color_for_style(style: &str) -> Option<Hsla> {
        match style {
            "keyword" => Some(rgb(0x569cd6).into()),
            "string" => Some(rgb(0xce9178).into()),
            "comment" => Some(rgb(0x6a9955).into()),
            "number" => Some(rgb(0xb5cea8).into()),
            "class" => Some(rgb(0x4ec9b0).into()),
            "method" => Some(rgb(0x9cdcfe).into()),
            "variable" => Some(rgb(0x9b9bc8).into()),
            "punctuation" => Some(rgb(0xd69d85).into()),
            "annotation" => Some(rgb(0xfffd9b).into()),
            "type" => Some(rgb(0x4ec9b0).into()),
            "preprocessor" => Some(rgb(0xc586c0).into()),
            "function" => Some(rgb(0xdcdcaa).into()),
            _ => None,
        }
    }

    fn shape_line(window: &Window, text: &str, color: Hsla, font_size: Pixels) -> ShapedLine {
        let style = window.text_style();
        let run = TextRun {
            len: text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        window.text_system().shape_line(
            SharedString::from(text.to_string()),
            font_size,
            &[run],
            None,
        )
    }

    fn shape_code_line(
        window: &Window,
        text: &str,
        font_size: Pixels,
        highlights: &[(std::ops::Range<usize>, Hsla)],
    ) -> ShapedLine {
        let mut runs = Vec::new();
        let mut last_end = 0;
        let style = window.text_style();
        for (range, color) in highlights {
            if range.start > last_end {
                runs.push(TextRun {
                    len: range.start - last_end,
                    font: style.font(),
                    color: rgb(0xcccccc).into(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
            runs.push(TextRun {
                len: range.end - range.start,
                font: style.font(),
                color: *color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
            last_end = range.end;
        }
        if last_end < text.len() {
            runs.push(TextRun {
                len: text.len() - last_end,
                font: style.font(),
                color: rgb(0xcccccc).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        }
        window.text_system().shape_line(
            SharedString::from(text.to_string()),
            font_size,
            &runs,
            None,
        )
    }

    fn code_highlights_for_line(
        spans: &[HighlightSpan],
        style_cache: &HashMap<u32, Hsla>,
        line_start_char: usize,
        line_text: &str,
    ) -> Vec<(std::ops::Range<usize>, Hsla)> {
        let mut result = Vec::new();
        let line_len_chars = line_text.chars().count();
        let line_end_char = line_start_char + line_len_chars;
        let mut char_indices = line_text.char_indices().peekable();
        let mut current_char_idx = 0;
        for span in spans {
            let s = span.start_index as usize;
            let e = span.end_index as usize;
            if s >= line_end_char { break; }
            if e > line_start_char && s < line_end_char {
                let start_in_line = s.max(line_start_char) - line_start_char;
                let end_in_line = e.min(line_end_char) - line_start_char;
                if start_in_line < end_in_line {
                    while current_char_idx < start_in_line {
                        char_indices.next();
                        current_char_idx += 1;
                    }
                    let start_b = char_indices.peek().map(|(b, _)| *b).unwrap_or(line_text.len());
                    while current_char_idx < end_in_line {
                        char_indices.next();
                        current_char_idx += 1;
                    }
                    let end_b = char_indices.peek().map(|(b, _)| *b).unwrap_or(line_text.len());
                    if start_b < end_b {
                        if let Some(color) = style_cache.get(&span.style_id) {
                            result.push((start_b..end_b, *color));
                        }
                    }
                }
            }
        }
        let mut normalized: Vec<(std::ops::Range<usize>, Hsla)> = Vec::with_capacity(result.len());
        let mut last_end = 0usize;
        let line_len = line_text.len();
        for (range, color) in result {
            let mut start = range.start.min(line_len);
            let end = range.end.min(line_len);
            if start < last_end { start = last_end; }
            if start < end {
                if let Some(last) = normalized.last_mut() {
                    if last.0.end == start && last.1 == color {
                        last.0.end = end;
                        last_end = end;
                        continue;
                    }
                }
                normalized.push((start..end, color));
                last_end = end;
            }
        }
        normalized
    }
}

impl Render for MarkdownViewer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let entity = _cx.entity();
        let view_for_scroll = entity.clone();
        let c = canvas(
            |bounds, _window, _cx| bounds,
            move |bounds, _layout, window, cx| {
                window.paint_quad(fill(bounds, rgb(0xff2d353b)));
                let font_size = px(13.0);
                let line_height = font_size * 1.6;
                let (scroll, mut content_h) = {
                    let viewer = entity.read(cx);
                    (viewer.scroll_offset_y, viewer.content_height)
                };
                let y0 = bounds.top() + px(8.0);
                let mut y_content = y0;
                let mut y_paint = y0 + scroll;
                let left = bounds.left() + px(16.0);
                let (blocks, style_cache) = {
                    let viewer = entity.read(cx);
                    (viewer.blocks.clone(), viewer.style_cache.clone())
                };
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    for b in &blocks {
                        match b {
                            Block::Heading(level, text) => {
                                let scale = match level {
                                    1 => 1.8,
                                    2 => 1.6,
                                    3 => 1.4,
                                    4 => 1.2,
                                    _ => 1.1,
                                };
                                let fs = font_size * scale as f32;
                                let lh = fs * 1.5;
                                let line = Self::shape_line(window, &text, rgb(0xffe6e0d9).into(), fs);
                                let origin = point(left, y_paint);
                                let _ = line.paint(origin, lh, window, cx);
                                y_paint += lh + px(6.0);
                                y_content += lh + px(6.0);
                            }
                            Block::Paragraph(text) => {
                                for para_line in text.split('\n') {
                                    let line = Self::shape_line(
                                        window,
                                        para_line,
                                        rgb(0xffc9d1d9).into(),
                                        font_size,
                                    );
                                    let origin = point(left, y_paint);
                                    let _ = line.paint(origin, line_height, window, cx);
                                    y_paint += line_height;
                                    y_content += line_height;
                                }
                                y_paint += px(6.0);
                                y_content += px(6.0);
                            }
                            Block::Hr => {
                                let hr_bounds = Bounds::from_corners(
                                    point(left, y_paint + px(6.0)),
                                    point(bounds.right() - px(16.0), y_paint + px(7.0)),
                                );
                                window.paint_quad(fill(hr_bounds, rgb(0xff3c474d)));
                                y_paint += px(12.0);
                                y_content += px(12.0);
                            }
                            Block::CodeBlock(_lang, lines, spans) => {
                                let mut acc_chars = 0usize;
                                let bg_right = bounds.right() - px(16.0);
                                let block_bg = Bounds::from_corners(
                                    point(left - px(4.0), y_paint - px(2.0)),
                                    point(bg_right, y_paint + line_height * lines.len() + px(6.0)),
                                );
                                window.paint_quad(fill(block_bg, rgb(0xff232a2e)));
                                for l in lines {
                                    let hl = Self::code_highlights_for_line(
                                        &spans,
                                        &style_cache,
                                        acc_chars,
                                        &l,
                                    );
                                    let line = Self::shape_code_line(window, &l, font_size, &hl);
                                    let origin = point(left, y_paint);
                                    let _ = line.paint(origin, line_height, window, cx);
                                    acc_chars += l.chars().count() + 1;
                                    y_paint += line_height;
                                    y_content += line_height;
                                }
                                y_paint += px(8.0);
                                y_content += px(8.0);
                            }
                        }
                    }
                });
                let total_height = y_content - y0;
                content_h = total_height;
                // Clamp scroll offset based on content height
                let max_scroll_y = (content_h - bounds.size.height + line_height).max(px(0.0));
                let clamped_scroll = scroll.max(-max_scroll_y).min(px(0.0));
                entity.update(cx, |viewer, _| {
                    viewer.content_height = content_h;
                    viewer.scroll_offset_y = clamped_scroll;
                });
                // Draw simple scrollbar
                if content_h > bounds.size.height {
                    let viewport_h = bounds.size.height;
                    let ratio = viewport_h / content_h;
                    let thumb_h = (viewport_h * ratio).clamp(px(24.0), viewport_h - px(24.0));
                    let scroll_pos = if max_scroll_y > px(0.0) {
                        let t = -f32::from(clamped_scroll) / f32::from(max_scroll_y);
                        (viewport_h - thumb_h - px(8.0)) * t
                    } else {
                        px(0.0)
                    };
                    let thumb_bounds = Bounds::from_corners(
                        point(bounds.right() - px(6.0), bounds.top() + px(4.0) + scroll_pos),
                        point(bounds.right() - px(2.0), bounds.top() + px(4.0) + scroll_pos + thumb_h),
                    );
                    window.paint_quad(fill(thumb_bounds, rgba(0xffffff55)));
                }
            },
        )
        .w_full()
        .h_full();
        div()
            .w_full()
            .h_full()
            .child(c)
            .on_scroll_wheel(move |event, _window, cx| {
                let delta = event.delta.pixel_delta(px(20.0)).y;
                view_for_scroll.update(cx, |viewer, cx_inner| {
                    viewer.scroll_offset_y -= delta;
                    cx_inner.notify();
                });
            })
    }
}
