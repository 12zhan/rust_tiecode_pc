use gpui::*;
use lru::LruCache;
use ropey::Rope;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use similar::TextDiff;
use std::process::Command;
use url::Url;

// Value and Url removed

pub mod completion;
pub mod core;
pub mod grammar;
pub mod layout;
pub mod lsp_integration;
pub mod undo;

#[cfg(test)]
mod tests;

use crate::editor::grammar::{
    CPP_GRAMMAR,
    CMAKE_GRAMMAR,
    CSS_GRAMMAR,
    HTML_GRAMMAR,
    JAVA_GRAMMAR,
    JAVASCRIPT_GRAMMAR,
    JIESHENG_GRAMMAR,
    JSON_GRAMMAR,
    MARKDOWN_GRAMMAR,
    PYTHON_GRAMMAR,
    RUST_GRAMMAR,
    SHELL_GRAMMAR,
    TOML_GRAMMAR,
    TYPESCRIPT_GRAMMAR,
    YAML_GRAMMAR,
};
use crate::editor::lsp_integration::{LspManager, default_doc_uri};

use self::core::{EditorCore, Selection};
use self::layout::EditorLayout;
use tiecode::sweetline::{Document, DocumentAnalyzer, Engine, HighlightSpan};

actions!(
    code_editor,
    [
        Backspace,
        Delete,
        DeleteLine,
        Enter,
        Left,
        Right,
        Up,
        Down,
        Tab,
        ShiftTab,
        CtrlShiftTab,
        SelectAll,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        ToggleFind,
        FindNext,
        FindPrev,
        CancelFind,
        Escape,
        GoToDefinition,
        SignatureHelp,
        FormatDocument
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationColor {
    Gray,
    #[allow(dead_code)]
    Yellow,
    #[allow(dead_code)]
    Red,
    #[allow(dead_code)]
    Custom(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GitDiffStatus {
    Added,
    Modified,
    Deleted,
}

impl DecorationColor {
    fn rgba(self) -> Rgba {
        match self {
            Self::Gray => rgba(0x928374FF), // Gruvbox gray #928374
            Self::Yellow => rgba(0xd79921FF), // Gruvbox yellow #d79921
            Self::Red => rgba(0xcc241dFF), // Gruvbox red #cc241d
            Self::Custom(c) => rgba(c),
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Red => 3,
            Self::Yellow => 2,
            Self::Gray => 1,
            Self::Custom(_) => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Decoration {
    pub range: Range<usize>,
    pub color: DecorationColor,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
struct HoverPopup {
    text: String,
    position: Point<Pixels>,
    color: DecorationColor,
}

pub enum CodeEditorEvent {
    OpenFile(PathBuf),
}

impl EventEmitter<CodeEditorEvent> for CodeEditor {}

#[derive(Clone, Debug)]
pub struct CodeLine {
    pub shaped: ShapedLine,
    pub map_orig_to_expanded: Vec<usize>,
}

impl CodeLine {
    pub fn x_for_index(&self, index: usize) -> Pixels {
        let expanded_index = if index >= self.map_orig_to_expanded.len() {
             *self.map_orig_to_expanded.last().unwrap_or(&0)
        } else {
             self.map_orig_to_expanded[index]
        };
        self.shaped.x_for_index(expanded_index)
    }

    pub fn paint(
        &self,
        origin: Point<Pixels>,
        line_height: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        self.shaped.paint(origin, line_height, window, cx)
    }

    pub fn index_for_x(&self, x: Pixels) -> Option<usize> {
        let expanded_idx = self.shaped.index_for_x(x)?;
        match self.map_orig_to_expanded.binary_search(&expanded_idx) {
            Ok(i) => Some(i),
            Err(i) => {
                if i == 0 { return Some(0); }
                if i >= self.map_orig_to_expanded.len() { return Some(self.map_orig_to_expanded.len() - 1); }
                
                let prev = self.map_orig_to_expanded[i - 1];
                let next = self.map_orig_to_expanded[i];
                if expanded_idx - prev < next - expanded_idx {
                    Some(i - 1)
                } else {
                    Some(i)
                }
            }
        }
    }
}

pub struct CodeEditor {
    pub focus_handle: FocusHandle,
    pub core: EditorCore,
    pub layout: EditorLayout,
    render_cache: Arc<Mutex<LruCache<String, CodeLine>>>,
    dragging_scrollbar: bool,
    drag_start_y: Option<Pixels>,
    scroll_start_y: Option<Pixels>,
    sweetline_engine: Arc<Engine>,
    sweetline_document: Option<Document>,
    sweetline_analyzer: Option<DocumentAnalyzer>,
    cached_highlights: Vec<HighlightSpan>,
    style_cache: HashMap<u32, Hsla>,
    decorations: Vec<Decoration>,
    hover_popup: Option<HoverPopup>,
    pub lsp_manager: LspManager,
    completion_scroll_offset: f32,
    pub git_diff_map: HashMap<usize, GitDiffStatus>,
    pub git_base_content: Option<String>,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>, file_path: Option<PathBuf>) -> Self {
        let engine = Arc::new(Engine::new(true));
        engine
            .compile_json(CPP_GRAMMAR)
            .expect("Failed to compile CPP grammar");
        engine
            .compile_json(RUST_GRAMMAR)
            .expect("Failed to compile Rust grammar");
        engine
            .compile_json(JSON_GRAMMAR)
            .expect("Failed to compile JSON grammar");
        engine
            .compile_json(CMAKE_GRAMMAR)
            .expect("Failed to compile CMake grammar");
        engine
            .compile_json(TOML_GRAMMAR)
            .expect("Failed to compile TOML grammar");
        engine
            .compile_json(YAML_GRAMMAR)
            .expect("Failed to compile YAML grammar");
        engine
            .compile_json(PYTHON_GRAMMAR)
            .expect("Failed to compile Python grammar");
        engine
            .compile_json(JAVASCRIPT_GRAMMAR)
            .expect("Failed to compile JavaScript grammar");
        engine
            .compile_json(JAVA_GRAMMAR)
            .expect("Failed to compile Java grammar");
        engine
            .compile_json(TYPESCRIPT_GRAMMAR)
            .expect("Failed to compile TypeScript grammar");
        engine
            .compile_json(HTML_GRAMMAR)
            .expect("Failed to compile HTML grammar");
        engine
            .compile_json(CSS_GRAMMAR)
            .expect("Failed to compile CSS grammar");
        engine
            .compile_json(MARKDOWN_GRAMMAR)
            .expect("Failed to compile Markdown grammar");
        engine
            .compile_json(SHELL_GRAMMAR)
            .expect("Failed to compile Shell grammar");
        engine
            .compile_json(JIESHENG_GRAMMAR)
            .expect("Failed to compile 结绳 grammar");

        let default_path = file_path.unwrap_or_else(|| std::env::temp_dir().join("untitled.t"));
        let doc_uri = default_doc_uri(&default_path);
        let doc = Document::new(&doc_uri, "");
        let analyzer = engine.load_document(&doc);

        let mut editor = Self {
            focus_handle: cx.focus_handle(),
            core: EditorCore::new(),
            layout: EditorLayout::new(),
            render_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))),
            dragging_scrollbar: false,
            drag_start_y: None,
            scroll_start_y: None,
            sweetline_engine: engine,
            sweetline_document: Some(doc),
            sweetline_analyzer: Some(analyzer),
            cached_highlights: Vec::new(),
            style_cache: HashMap::new(),
            decorations: Vec::new(),
            hover_popup: None,
            lsp_manager: LspManager::new(doc_uri),
            completion_scroll_offset: 0.0,
            git_diff_map: HashMap::new(),
            git_base_content: None,
        };

        editor.init_lsp_and_spawn_loop(cx);
        editor.fetch_git_base_content(cx);

        editor
    }

    fn init_lsp_and_spawn_loop(&mut self, _cx: &mut Context<Self>) {
        self.lsp_manager.initialize(&self.core.content.to_string());
    }

    pub fn fetch_git_base_content(&mut self, cx: &mut Context<Self>) {
        if let Ok(url) = Url::parse(&self.lsp_manager.doc_uri) {
            if let Ok(path) = url.to_file_path() {
                if let Some(parent) = path.parent() {
                    if let Some(file_name) = path.file_name() {
                        let output = Command::new("git")
                            .arg("show")
                            .arg(format!("HEAD:./{}", file_name.to_string_lossy()))
                            .current_dir(parent)
                            .output();

                        if let Ok(output) = output {
                            if output.status.success() {
                                if let Ok(content) = String::from_utf8(output.stdout) {
                                    self.git_base_content = Some(content);
                                    self.update_git_diff(cx);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        self.git_base_content = None;
        self.update_git_diff(cx);
    }

    pub fn update_git_diff(&mut self, cx: &mut Context<Self>) {
        self.git_diff_map.clear();
        
        if let Some(base) = &self.git_base_content {
            let current = self.core.content.to_string();
            let diff = TextDiff::from_lines(base, &current);
            
            for op in diff.ops() {
                match op.tag() {
                    similar::DiffTag::Delete => {
                        // println!("Diff: Deleted at {}", op.new_range().start);
                        if op.new_range().start <= self.core.content.len_lines() {
                             self.git_diff_map.insert(op.new_range().start, GitDiffStatus::Deleted);
                        }
                    }
                    similar::DiffTag::Insert => {
                        // println!("Diff: Inserted at {:?}", op.new_range());
                        for i in op.new_range() {
                            self.git_diff_map.insert(i, GitDiffStatus::Added);
                        }
                    }
                    similar::DiffTag::Replace => {
                        // println!("Diff: Modified at {:?}", op.new_range());
                        for i in op.new_range() {
                            self.git_diff_map.insert(i, GitDiffStatus::Modified);
                        }
                    }
                    similar::DiffTag::Equal => {}
                }
            }
        }
        cx.notify();
    }

    pub fn perform_undo(&mut self, cx: &mut Context<Self>) {
        self.core.undo();
        self.sync_sweetline_document(cx);
        cx.notify();
    }

    pub fn perform_redo(&mut self, cx: &mut Context<Self>) {
        self.core.redo();
        self.sync_sweetline_document(cx);
        cx.notify();
    }

    pub fn perform_select_all(&mut self, cx: &mut Context<Self>) {
        self.core.select_all();
        self.core.completion_active = false;
        cx.notify();
    }

    pub fn perform_copy(&mut self, cx: &mut Context<Self>) {
        let mut texts = Vec::new();
        for selection in &self.core.selections {
            if !selection.is_empty() {
                texts.push(self.core.content.byte_slice(selection.range()).to_string());
            }
        }
        if texts.is_empty() {
            return;
        }
        let text = texts.join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    pub fn perform_cut(&mut self, cx: &mut Context<Self>) {
        self.perform_copy(cx);
        self.core.delete_selection();
        self.sync_sweetline_document(cx);
        cx.notify();
    }

    pub fn perform_paste(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.insert_text(&text, cx);
            }
        }
    }


    fn paint_soft_shadow(window: &mut Window, bounds: Bounds<Pixels>, corner_radius: Pixels) {
        let steps = 10;
        let max_blur = px(24.0);
        let max_alpha = 0.12;
        let offset_y = px(4.0);

        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let spread = max_blur * t;
            let current_offset_y = offset_y * t;
            
            let shadow_bounds = Bounds::from_corners(
                point(bounds.left() - spread, bounds.top() - spread + current_offset_y),
                point(bounds.right() + spread, bounds.bottom() + spread + current_offset_y),
            );
            
            let decay_factor = (1.0 - t).powi(2);
            let current_alpha = max_alpha * decay_factor;
            
            if current_alpha > 0.0 {
                let color = gpui::hsla(0.0, 0.0, 0.0, current_alpha);
                let radius = corner_radius + spread;
                
                let mut quad = fill(shadow_bounds, color);
                quad.corner_radii = Corners::all(radius);
                window.paint_quad(quad);
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_decorations(&mut self, decorations: Vec<Decoration>, cx: &mut Context<Self>) {
        self.decorations = decorations;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn clear_decorations(&mut self, cx: &mut Context<Self>) {
        self.decorations.clear();
        self.hover_popup = None;
        cx.notify();
    }

    pub fn open_file(&mut self, path: PathBuf, content: String, cx: &mut Context<Self>) {
        let new_uri = default_doc_uri(&path);
        
        if new_uri == self.lsp_manager.doc_uri {
            self.set_content(content, cx);
            return;
        }

        // Detect project root and restart LSP if needed
        let new_root_path = LspManager::detect_project_root(&path);
        let new_root_uri = default_doc_uri(&new_root_path);
        
        if new_root_uri != self.lsp_manager.root_uri {
            self.lsp_manager.restart(new_root_path, &content);
        }

        // Clean up old document
        let _ = self.sweetline_engine.remove_document(&self.lsp_manager.doc_uri);
        
        // Register new file with LSP
        self.lsp_manager.update_doc_uri(new_uri, &content);
        
        self.core.content = Rope::from(content.clone());
        self.core.set_cursor(0);
        self.decorations.clear();
        self.hover_popup = None;
        self.fetch_git_base_content(cx);
        self.sync_sweetline_document(cx);

        cx.notify();
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.core.content = Rope::from(content.clone());
        self.sync_sweetline_document(cx);
        self.core.set_cursor(0);
        
        self.lsp_manager.notify_change(&content);
        
        cx.notify();
    }

    pub fn set_cursor(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.set_cursor(index);
        self.core.completion_active = false;
        self.hover_popup = None;
        cx.notify();
    }

    pub fn select_to(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.select_to(index);
        self.core.completion_active = false;
        cx.notify();
    }

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.core.insert_text(text);
        self.sync_sweetline_document(cx);
        self.notify_lsp_change(text);
        self.update_completion(cx);
        cx.notify();
    }

    fn notify_lsp_change(&mut self, _text: &str) {
        let content = self.core.content.to_string();
        self.lsp_manager.notify_change(&content);
    }

    #[allow(dead_code)]
    pub fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.core.delete_range(range);
        self.sync_sweetline_document(cx);
        self.notify_lsp_change("");
        self.update_completion(cx);
        cx.notify();
    }

    fn process_lsp_messages(&mut self, _cx: &mut Context<Self>) {
        // LSP functionality removed
    }

    fn scroll_to_cursor(&mut self, cx: &mut Context<Self>) {
        if let Some(bounds) = self.layout.last_bounds {
            let index = self.core.primary_selection().head;
            let (line, _, _) = Self::line_col_for_index(&self.core.content, index);
            
            let line_height = self.layout.line_height();
            let line_top = line_height * line as f32;
            let line_bottom = line_top + line_height;
            
            let scroll_top = -self.layout.scroll_offset.y;
            let scroll_bottom = scroll_top + bounds.size.height;
            
            if line_top < scroll_top {
                self.layout.scroll_offset.y = -line_top;
            } else if line_bottom > scroll_bottom {
                self.layout.scroll_offset.y = -(line_bottom - bounds.size.height);
            }
            cx.notify();
        }
    }

    fn point_for_index(&mut self, index: usize) -> Point<Pixels> {
        if let Some(bounds) = self.layout.last_bounds {
            let (line, _, line_start) = Self::line_col_for_index(&self.core.content, index);
            let line_slice = self.core.content.line(line);
            let mut line_text_string = line_slice.to_string();
            if line_text_string.ends_with('\n') {
                line_text_string.pop();
                if line_text_string.ends_with('\r') {
                    line_text_string.pop();
                }
            }
            
            let key = format!("{}:{}", line, line_text_string);
            let x_offset = if let Ok(mut cache) = self.render_cache.lock() {
                if let Some(line) = cache.get(&key) {
                     let local_index = index.saturating_sub(line_start).min(line_text_string.len());
                     line.x_for_index(local_index)
                } else {
                    px(0.0)
                }
            } else {
                px(0.0)
            };
            
            let line_count = self.core.content.len_lines().max(1);
            let max_digits = line_count.to_string().len();
            let text_x = self.layout.text_x(bounds, max_digits);
            
            let y = self.layout.line_y(bounds, line);
            
            point(text_x + x_offset, y)
        } else {
            point(px(0.0), px(0.0))
        }
    }

    fn update_completion(&mut self, cx: &mut Context<Self>) {
        self.process_lsp_messages(cx);

        let primary = self.core.primary_selection();
        if primary.is_empty() {
            let cursor = primary.head;
            
            // --- C++ Keyword Completion (Fixed) ---
            let content = &self.core.content;
            let len_bytes = content.len_bytes();
            
            if cursor > len_bytes {
                return;
            }

            let mut word_start = cursor;
            let mut current_idx = cursor;
            let len_chars = content.len_chars();

            while current_idx > 0 {
                if current_idx > len_bytes {
                    break;
                }

                let char_idx = content.byte_to_char(current_idx);
                if char_idx == 0 && current_idx > 0 {
                     break;
                }
                
                let prev_char_idx = char_idx.saturating_sub(1);
                if prev_char_idx >= len_chars {
                    break;
                }

                let ch = content.char(prev_char_idx);
                let char_len = ch.len_utf8();
                
                if current_idx < char_len {
                    break;
                }
                let prev_byte_idx = current_idx - char_len;

                if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                    word_start = current_idx;
                    break;
                }
                current_idx = prev_byte_idx;
                word_start = current_idx;
            }
            
            let word_start_char = content.byte_to_char(word_start);
            let cursor_char = content.byte_to_char(cursor);
            let prefix = content.slice(word_start_char..cursor_char).to_string();
            
            if !prefix.is_empty() {
                // Use LSP for completion
                let line_idx = content.byte_to_line(cursor);
                let line_start_byte = content.line_to_byte(line_idx);
                let char_idx = content.byte_to_char(cursor).saturating_sub(content.byte_to_char(line_start_byte));

                if let Some(mut items) = self.lsp_manager.completion(line_idx, char_idx, cursor, &prefix, "") {
                    items.retain(|item| item.label.starts_with(&prefix));
                    
                    if !items.is_empty() {
                        self.core.completion_items = items;
                        self.core.completion_active = true;
                        self.core.completion_index = 0;
                        self.completion_scroll_offset = 0.0;
                        cx.notify();
                        return;
                    }
                }
            }
            // --- End C++ Keyword Completion ---

// JFLSP integration removed
        }
    }

    fn confirm_completion(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = self.core.completion_items.get(self.core.completion_index) {
            let label = item.label.clone();

            let primary = self.core.primary_selection();
            let cursor = primary.head;
            let content = &self.core.content;
            let len_bytes = content.len_bytes();
            let len_chars = content.len_chars();
            
            let mut word_start = cursor;
            let mut current_idx = cursor;
            
            while current_idx > 0 {
                if current_idx > len_bytes {
                    break;
                }

                let char_idx = content.byte_to_char(current_idx);
                if char_idx == 0 && current_idx > 0 {
                    break;
                }
                
                let prev_char_idx = char_idx.saturating_sub(1);
                if prev_char_idx >= len_chars {
                    break;
                }

                let ch = content.char(prev_char_idx);
                let char_len = ch.len_utf8();
                
                if current_idx < char_len {
                    break;
                }
                let prev_byte_idx = current_idx - char_len;

                if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                    word_start = current_idx;
                    break;
                }
                current_idx = prev_byte_idx;
                word_start = current_idx;
            }

            self.core.replace_range(word_start..cursor, &label);
            self.sync_sweetline_document(cx);
            self.update_completion(cx);

            self.core.completion_active = false;
            self.core.completion_items.clear();
            self.core.completion_index = 0;
            cx.notify();
        }
    }

    fn backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        // Expand empty selections to include previous char
        for selection in self.core.selections.iter_mut() {
            if selection.is_empty() {
                let cursor = selection.head;
                let prev = Self::prev_char_index(&self.core.content, cursor);
                // Modify selection to cover the character to be deleted
                // Note: we set anchor=prev, head=cursor (forward selection) or vice versa.
                // replace_selections uses .range(), so order doesn't matter for deletion content,
                // but we want to ensure we delete the range prev..cursor.
                *selection = Selection::new(prev, cursor);
            }
        }
        self.core.delete_selection();
        self.sync_sweetline_document(cx);
        self.update_completion(cx);
        cx.notify();
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        // Expand empty selections to include next char
        for selection in self.core.selections.iter_mut() {
            if selection.is_empty() {
                let cursor = selection.head;
                let next = Self::next_char_index(&self.core.content, cursor);
                *selection = Selection::new(cursor, next);
            }
        }
        self.core.delete_selection();
        self.sync_sweetline_document(cx);
        self.update_completion(cx);
        cx.notify();
    }

    fn delete_line(&mut self, _: &DeleteLine, _window: &mut Window, cx: &mut Context<Self>) {
        let mut ranges_to_delete = Vec::new();

        for selection in &self.core.selections {
            let cursor = selection.range().start;
            let line_idx = self.core.content.byte_to_line(cursor);
            let line_start = self.core.content.line_to_byte(line_idx);

            let next_line_idx = line_idx + 1;
            let range_end = if next_line_idx < self.core.content.len_lines() {
                self.core.content.line_to_byte(next_line_idx)
            } else {
                self.core.content.len_bytes()
            };

            let mut range_start = line_start;

            // If last line, try to delete previous newline
            if next_line_idx >= self.core.content.len_lines() && range_start > 0 {
                range_start -= 1;
                if range_start > 0 && self.core.content.byte(range_start - 1) == b'\r' {
                    range_start -= 1;
                }
            }
            ranges_to_delete.push(range_start..range_end);
        }

        // We need to merge ranges if they overlap to avoid double deletion issues if we did it sequentially?
        // Actually replace_selections expects selections.
        // We can just construct selections from these ranges and call replace_selections("").
        // But replace_selections expects US to update selections after.
        // It's easier to use replace_selections.

        self.core.selections = ranges_to_delete
            .into_iter()
            .map(|r| Selection::new(r.start, r.end))
            .collect();
        self.core.merge_selections(); // Merge overlapping lines

        self.core.delete_selection();
        self.sync_sweetline_document(cx);
        cx.notify();
    }

    fn enter(&mut self, _: &Enter, _window: &mut Window, cx: &mut Context<Self>) {
        if self.core.marked_range.is_some() {
            return;
        }
        if self.core.completion_active {
            self.confirm_completion(cx);
            return;
        }
        self.insert_text("\n", cx);
    }

    fn tab(&mut self, _: &Tab, _window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            self.confirm_completion(cx);
            return;
        }
        self.insert_text("    ", cx);
    }

    fn shift_tab(&mut self, _: &ShiftTab, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("special: shift-tab");
    }

    #[allow(dead_code)]
    fn ctrl_shift_tab(&mut self, _: &CtrlShiftTab, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("special: ctrl-shift-tab");
    }

    fn undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        self.core.undo();
        self.sync_sweetline_document(cx);
        self.update_completion(cx);
        cx.notify();
    }

    fn redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        self.core.redo();
        self.sync_sweetline_document(cx);
        self.update_completion(cx);
        cx.notify();
    }

    fn select_all(&mut self, _: &SelectAll, _window: &mut Window, cx: &mut Context<Self>) {
        self.core.select_all();
        self.core.completion_active = false;
        cx.notify();
    }

    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        let mut texts = Vec::new();
        for selection in &self.core.selections {
            if !selection.is_empty() {
                texts.push(self.core.content.byte_slice(selection.range()).to_string());
            }
        }
        if texts.is_empty() {
            return;
        }
        // Join with newlines for now
        let text = texts.join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, _window, cx);
        self.core.delete_selection();
        self.sync_sweetline_document(cx);
        cx.notify();
    }

    fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.insert_text(&text, cx);
            }
        }
    }

    // Find actions stubs to prevent crash if keybindings exist
    fn toggle_find(&mut self, _: &ToggleFind, _: &mut Window, _: &mut Context<Self>) {}
    fn find_next(&mut self, _: &FindNext, _: &mut Window, _: &mut Context<Self>) {}
    fn find_prev(&mut self, _: &FindPrev, _: &mut Window, _: &mut Context<Self>) {}
    fn cancel_find(&mut self, _: &CancelFind, _: &mut Window, _: &mut Context<Self>) {}

    fn go_to_definition(&mut self, _: &GoToDefinition, _: &mut Window, _cx: &mut Context<Self>) {
        // LSP functionality removed
    }

    fn signature_help(&mut self, _: &SignatureHelp, _: &mut Window, _cx: &mut Context<Self>) {
        // LSP functionality removed
    }

    fn format_document(&mut self, _: &FormatDocument, _: &mut Window, _cx: &mut Context<Self>) {
        // LSP functionality removed
    }

    fn escape(&mut self, _: &Escape, _: &mut Window, cx: &mut Context<Self>) {
        self.core.selections = vec![self.core.selections[0].clone()];
        self.core.completion_active = false;
        self.hover_popup = None;
        cx.notify();
    }

    fn on_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        if event.capslock.on {
            println!("special: caps-lock on");
        } else {
            println!("special: caps-lock off");
        }
    }

    fn move_left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        let content = &self.core.content;
        let shift = window.modifiers().shift;

        for selection in self.core.selections.iter_mut() {
            let head = selection.head;

            if shift {
                let prev = Self::prev_char_index(content, head);
                selection.head = prev;
            } else {
                if !selection.is_empty() {
                    let new_pos = selection.range().start;
                    *selection = Selection::new(new_pos, new_pos);
                } else {
                    let prev = Self::prev_char_index(content, head);
                    *selection = Selection::new(prev, prev);
                }
            }
            selection.preferred_column = None;
        }
        self.core.merge_selections();
        cx.notify();
    }

    fn move_right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        let content = &self.core.content;
        let shift = window.modifiers().shift;

        for selection in self.core.selections.iter_mut() {
            let head = selection.head;

            if shift {
                let next = Self::next_char_index(content, head);
                selection.head = next;
            } else {
                if !selection.is_empty() {
                    let new_pos = selection.range().end;
                    *selection = Selection::new(new_pos, new_pos);
                } else {
                    let next = Self::next_char_index(content, head);
                    *selection = Selection::new(next, next);
                }
            }
            selection.preferred_column = None;
        }
        self.core.merge_selections();
        cx.notify();
    }

    fn ensure_completion_visible(&mut self) {
        let max_visible_items = 10;
        let current_scroll = self.completion_scroll_offset as usize;
        let index = self.core.completion_index;
        
        if index < current_scroll {
            self.completion_scroll_offset = index as f32;
        } else if index >= current_scroll + max_visible_items {
            self.completion_scroll_offset = (index - max_visible_items + 1) as f32;
        }
    }

    fn move_up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            if self.core.completion_index > 0 {
                self.core.completion_index -= 1;
                self.ensure_completion_visible();
                cx.notify();
            }
            return;
        }

        let content = &self.core.content;
        let shift = window.modifiers().shift;

        // We need to calculate new selections
        let mut new_selections = Vec::new();

        for selection in &self.core.selections {
            let head = selection.head;
            let cursor = if shift {
                head
            } else {
                if !selection.is_empty() {
                    selection.range().start
                } else {
                    head
                }
            };

            let (line, col, _) = Self::line_col_for_index(content, cursor);
            let preferred = selection.preferred_column.unwrap_or(col);

            let target_line = line.saturating_sub(1);
            let new_index = Self::index_for_line_col(content, target_line, preferred);

            let mut new_sel = if shift {
                let mut s = selection.clone();
                s.head = new_index;
                s
            } else {
                Selection::new(new_index, new_index)
            };
            new_sel.preferred_column = Some(preferred);
            new_selections.push(new_sel);
        }
        self.core.selections = new_selections;
        self.core.merge_selections();
        cx.notify();
    }

    fn move_down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            if self.core.completion_index < self.core.completion_items.len().saturating_sub(1) {
                self.core.completion_index += 1;
                cx.notify();
            }
            return;
        }

        let content = &self.core.content;
        let shift = window.modifiers().shift;

        let mut new_selections = Vec::new();

        for selection in &self.core.selections {
            let head = selection.head;
            let cursor = if shift {
                head
            } else {
                if !selection.is_empty() {
                    selection.range().end
                } else {
                    head
                }
            };

            let (line, col, _) = Self::line_col_for_index(content, cursor);
            let preferred = selection.preferred_column.unwrap_or(col);
            let max_line = content.len_lines().saturating_sub(1);
            let target_line = (line + 1).min(max_line);
            let new_index = Self::index_for_line_col(content, target_line, preferred);

            let mut new_sel = if shift {
                let mut s = selection.clone();
                s.head = new_index;
                s
            } else {
                Selection::new(new_index, new_index)
            };
            new_sel.preferred_column = Some(preferred);
            new_selections.push(new_sel);
        }
        self.core.selections = new_selections;
        self.core.merge_selections();
        cx.notify();
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if _window.modifiers().control {
            let old_font_size = self.layout.font_size;
            let old_line_height = self.layout.line_height();
            let old_scroll_offset = self.layout.scroll_offset;
            let delta = event.delta.pixel_delta(px(10.0)).y;
            self.layout.zoom(delta);
            let new_font_size = self.layout.font_size;
            let new_line_height = self.layout.line_height();

            let x_ratio = if old_font_size > px(0.0) {
                new_font_size / old_font_size
            } else {
                1.0
            };
            let y_ratio = if old_line_height > px(0.0) {
                new_line_height / old_line_height
            } else {
                1.0
            };

            self.layout.scroll_offset.x = old_scroll_offset.x * x_ratio;
            self.layout.scroll_offset.y = old_scroll_offset.y * y_ratio;

            if let Ok(mut cache) = self.render_cache.lock() {
                cache.clear();
            }

            let bounds = self.layout.last_bounds.unwrap_or_default();
            let view_size = bounds.size;

            let line_count = self.core.content.len_lines().max(1);
            let total_height = self.layout.line_height() * line_count as f32;
            let max_scroll_y =
                (total_height - view_size.height + self.layout.line_height()).max(px(0.0));

            let max_line_len = self
                .core
                .content
                .lines()
                .map(|l| l.len_bytes())
                .max()
                .unwrap_or(0);
            let max_digits = line_count.to_string().len();
            let gutter_width = self.layout.gutter_width(max_digits);
            let char_width = self.layout.font_size * 0.75;
            let content_width = gutter_width + px(40.0) + (max_line_len as f32 * char_width);
            let max_scroll_x = (content_width - view_size.width).max(px(0.0));

            self.layout.scroll_offset.y = self.layout.scroll_offset.y.clamp(-max_scroll_y, px(0.0));
            self.layout.scroll_offset.x = self.layout.scroll_offset.x.clamp(-max_scroll_x, px(0.0));
        } else {
            let delta = event.delta.pixel_delta(px(20.0));

            let bounds = self.layout.last_bounds.unwrap_or_default();
            let view_size = bounds.size;

            let line_count = self.core.content.len_lines().max(1);
            let total_height = self.layout.line_height() * line_count as f32;
            let max_scroll_y =
                (total_height - view_size.height + self.layout.line_height()).max(px(0.0));

            let max_line_len = self
                .core
                .content
                .lines()
                .map(|l| l.len_bytes())
                .max()
                .unwrap_or(0);
            let max_digits = line_count.to_string().len();
            let gutter_width = self.layout.gutter_width(max_digits);
            let char_width = self.layout.font_size * 0.75;
            let content_width = gutter_width + px(40.0) + (max_line_len as f32 * char_width);
            let max_scroll_x = (content_width - view_size.width).max(px(0.0));

            self.layout.scroll(delta, point(max_scroll_x, max_scroll_y));
        }
        cx.notify();
    }

    fn paint_squiggly(
        window: &mut Window,
        x0: Pixels,
        x1: Pixels,
        y: Pixels,
        line_height: Pixels,
        color: Rgba,
    ) {
        if x1 <= x0 {
            return;
        }
        let amplitude = (line_height * 0.14).clamp(px(2.5), px(6.0));
        let period = (line_height * 0.9).clamp(px(10.0), px(18.0));
        let amplitude_f = f32::from(amplitude);
        let period_f = f32::from(period);
        let x0_f = f32::from(x0);
        let omega = std::f32::consts::TAU / period_f;
        let phase = (x0_f / period_f) * std::f32::consts::TAU;
        let vertical_shift = amplitude;

        let stroke_width = (line_height * 0.11).clamp(px(1.5), px(2.5));
        let sample_step = (line_height * 0.12).clamp(px(1.0), px(3.0));

        let mut builder = PathBuilder::stroke(stroke_width);
        let y0 = y + px(amplitude_f * (omega * x0_f + phase).sin()) + vertical_shift;
        builder.move_to(point(x0, y0));

        let mut x = x0 + sample_step;
        while x <= x1 {
            let yy =
                y + px(amplitude_f * (omega * f32::from(x) + phase).sin()) + vertical_shift;
            builder.line_to(point(x, yy));
            x += sample_step;
        }

        if x != x1 {
            let yy =
                y + px(amplitude_f * (omega * f32::from(x1) + phase).sin()) + vertical_shift;
            builder.line_to(point(x1, yy));
        }

        if let Ok(path) = builder.build() {
            window.paint_path(path, Background::from(color));
        }
    }

    fn expand_tabs(text: &str, tab_size: usize) -> (String, Vec<usize>) {
        let mut expanded = String::with_capacity(text.len());
        let mut map = Vec::with_capacity(text.len() + 1);
        let mut col = 0;

        for (_i, c) in text.char_indices() {
             let len = c.len_utf8();
             for _ in 0..len {
                 map.push(expanded.len());
             }

             if c == '\t' {
                 let count = tab_size - (col % tab_size);
                 for _ in 0..count { expanded.push(' '); }
                 col += count;
             } else {
                 expanded.push(c);
                 col += 1;
             }
        }
        map.push(expanded.len());
        (expanded, map)
    }

    fn get_cached_shape_line(
        &self,
        window: &Window,
        text: &str,
        font_size: Pixels,
        line_index: usize,
        line_start_byte: usize,
    ) -> CodeLine {
        let key = format!("{}:{}", line_index, text);

        if let Ok(mut cache) = self.render_cache.lock() {
            if let Some(line) = cache.get(&key) {
                return line.clone();
            }
        }

        let (expanded_text, map) = Self::expand_tabs(text, 4);
        let highlights = self.get_highlights_for_line(line_start_byte, text);
        
        let mut expanded_highlights = Vec::new();
        for (range, color) in highlights {
             let start = map.get(range.start).cloned().unwrap_or(expanded_text.len());
             let end = map.get(range.end).cloned().unwrap_or(expanded_text.len());
             expanded_highlights.push((start..end, color));
        }

        let shaped = Self::shape_code_line(window, &expanded_text, font_size, &expanded_highlights);
        
        let line = CodeLine {
             shaped,
             map_orig_to_expanded: map,
        };

        if let Ok(mut cache) = self.render_cache.lock() {
            cache.put(key, line.clone());
        }
        line
    }

    fn sync_sweetline_document(&mut self, cx: &mut Context<Self>) {
        let text = self.core.content.to_string();

        let _ = self.sweetline_engine.remove_document(&self.lsp_manager.doc_uri);
        self.sweetline_analyzer = None;
        self.sweetline_document = None;

        let doc = Document::new(&self.lsp_manager.doc_uri, &text);
        let analyzer = self.sweetline_engine.load_document(&doc);

        self.sweetline_document = Some(doc);
        self.sweetline_analyzer = Some(analyzer);

        self.update_highlights();
        self.update_git_diff(cx);
    }

    fn update_highlights(&mut self) {
        if let Some(analyzer) = &self.sweetline_analyzer {
            let result = analyzer.analyze();
            self.cached_highlights = DocumentAnalyzer::parse_result(&result, false);
            self.cached_highlights
                .sort_by(|a, b| (a.end_index, a.start_index).cmp(&(b.end_index, b.start_index)));

            // Update style cache
            for span in &self.cached_highlights {
                if !self.style_cache.contains_key(&span.style_id) {
                    if let Some(name) = self.sweetline_engine.get_style_name(span.style_id) {
                        // println!("Caching style: {} -> {}", span.style_id, name);
                        if let Some(color) = self.color_for_style(&name) {
                            self.style_cache.insert(span.style_id, color);
                        }
                    }
                }
            }

            // Invalidate render cache
            if let Ok(mut cache) = self.render_cache.lock() {
                cache.clear();
            }
        }
    }

    fn update_highlights_from_result(&mut self, result: Vec<i32>) {
        self.cached_highlights = DocumentAnalyzer::parse_result(&result, false);
        self.cached_highlights
            .sort_by(|a, b| (a.end_index, a.start_index).cmp(&(b.end_index, b.start_index)));
        for span in &self.cached_highlights {
            if !self.style_cache.contains_key(&span.style_id) {
                if let Some(name) = self.sweetline_engine.get_style_name(span.style_id) {
                    if let Some(color) = self.color_for_style(&name) {
                        self.style_cache.insert(span.style_id, color);
                    }
                }
            }
        }
        if let Ok(mut cache) = self.render_cache.lock() {
            cache.clear();
        }
    }

    fn get_highlights_for_line(
        &self,
        line_start_byte: usize,
        line_text: &str,
    ) -> Vec<(Range<usize>, Hsla)> {
        let mut result = Vec::new();
        
        let content_len = self.core.content.len_bytes();
        let safe_line_start_byte = line_start_byte.min(content_len);
        let line_start_char = self.core.content.byte_to_char(safe_line_start_byte);
        
        let line_char_len = line_text.chars().count();
        let line_end_char = line_start_char + line_char_len;

        // Binary search for the first span that ends after the line starts
        let start_idx = self.cached_highlights.partition_point(|span| {
            (span.end_index as usize) <= line_start_char
        });

        // Iterate through spans using a single pass over line characters
        let mut char_indices = line_text.char_indices().peekable();
        let mut current_char_idx = 0;
        
        for span in &self.cached_highlights[start_idx..] {
            let span_start_char = span.start_index as usize;
            let span_end_char = span.end_index as usize;

            if span_start_char >= line_end_char {
                break;
            }

            // Intersection check
            if span_end_char > line_start_char && span_start_char < line_end_char {
                let start_in_line_chars = span_start_char.max(line_start_char) - line_start_char;
                let end_in_line_chars = span_end_char.min(line_end_char) - line_start_char;

                if start_in_line_chars < end_in_line_chars {
                     // Find start_byte
                     while current_char_idx < start_in_line_chars {
                         char_indices.next();
                         current_char_idx += 1;
                     }
                     let start_byte = char_indices.peek().map(|(b, _)| *b).unwrap_or(line_text.len());

                     // Find end_byte
                     while current_char_idx < end_in_line_chars {
                         char_indices.next();
                         current_char_idx += 1;
                     }
                     let end_byte = char_indices.peek().map(|(b, _)| *b).unwrap_or(line_text.len());

                     if start_byte < end_byte {
                        if let Some(color) = self.style_cache.get(&span.style_id) {
                            result.push((start_byte..end_byte, *color));
                        }
                     }
                }
            }
        }

        // Result is already sorted by virtue of cached_highlights being sorted and sequential processing
        // Merging adjacent same-colored spans if needed
        let mut normalized: Vec<(Range<usize>, Hsla)> = Vec::with_capacity(result.len());
        let mut last_end = 0usize;
        let line_len = line_text.len();
        
        for (range, color) in result {
            let mut start = range.start.min(line_len);
            let end = range.end.min(line_len);
            
            if start < last_end {
                start = last_end;
            }
            if start < end {
                // Optimization: merge with previous if contiguous and same color
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



    fn clamp_to_char_boundary(text: &str, idx: usize) -> usize {
        let idx = idx.min(text.len());
        if text.is_char_boundary(idx) {
            return idx;
        }
        text.char_indices()
            .map(|(byte_idx, _)| byte_idx)
            .take_while(|&byte_idx| byte_idx < idx)
            .last()
            .unwrap_or(0)
    }

    fn color_for_style(&self, style: &str) -> Option<Hsla> {
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

    // Helper functions
    fn line_col_for_index(content: &Rope, index: usize) -> (usize, usize, usize) {
        if index > content.len_bytes() {
            eprintln!(
                "Warning: line_col_for_index out of bounds: {} > {}",
                index,
                content.len_bytes()
            );
            let len = content.len_bytes();
            let line_index = content.byte_to_line(len);
            let line_start = content.line_to_byte(line_index);
            let col = len - line_start;
            return (line_index, col, line_start);
        }
        let line_index = content.byte_to_line(index);
        let line_start = content.line_to_byte(line_index);
        let col = index - line_start;
        (line_index, col, line_start)
    }

    fn lsp_position_for_index(&self, index: usize) -> (usize, usize) {
        let content = &self.core.content;
        let len = content.len_bytes();
        let clamped = index.min(len);
        let line_index = content.byte_to_line(clamped);
        let line_start_byte = content.line_to_byte(line_index);
        let line_start_char = content.byte_to_char(line_start_byte);
        let index_char = content.byte_to_char(clamped);
        let col_utf16 = content.slice(line_start_char..index_char).len_utf16_cu();
        (line_index, col_utf16)
    }

    fn lsp_point_to_offset(&self, line: usize, char_utf16: usize) -> usize {
        let content = &self.core.content;
        if line >= content.len_lines() {
             return content.len_bytes();
        }
        let line_start_byte = content.line_to_byte(line);
        let line_slice = content.line(line);
        
        // Fast path: if char_utf16 is 0, return start of line
        if char_utf16 == 0 {
            return line_start_byte;
        }

        // Iterate chars to find byte offset matching utf16 length
        let mut current_utf16 = 0;
        let mut byte_offset_in_line = 0;
        
        for char in line_slice.chars() {
             if current_utf16 >= char_utf16 {
                 break;
             }
             let len = char.len_utf16();
             if current_utf16 + len > char_utf16 {
                 // pointing into middle of a char? return start of this char
                 break;
             }
             current_utf16 += len;
             byte_offset_in_line += char.len_utf8();
        }
        
        line_start_byte + byte_offset_in_line
    }

    fn index_for_line_col(content: &Rope, line: usize, col: usize) -> usize {
        if line >= content.len_lines() {
            return content.len_bytes();
        }
        let line_start = content.line_to_byte(line);
        let line_slice = content.line(line);
        let mut len = line_slice.len_bytes();
        if line + 1 < content.len_lines() {
            if line_slice.chars().last() == Some('\n') {
                len -= 1;
            }
        }
        line_start + col.min(len)
    }

    fn prev_char_index(content: &Rope, index: usize) -> usize {
        if index == 0 {
            return 0;
        }
        if index > content.len_bytes() {
            return content.len_bytes();
        }
        let char_idx = content.byte_to_char(index);
        if char_idx == 0 {
            return 0;
        }
        
        // Check for CRLF
        let prev_char = content.char(char_idx - 1);
        if prev_char == '\n' && char_idx > 1 {
            if content.char(char_idx - 2) == '\r' {
                return content.char_to_byte(char_idx - 2);
            }
        }
        
        content.char_to_byte(char_idx - 1)
    }

    fn next_char_index(content: &Rope, index: usize) -> usize {
        if index >= content.len_bytes() {
            return content.len_bytes();
        }
        let char_idx = content.byte_to_char(index);
        
        // Check for CRLF
        let curr_char = content.char(char_idx);
        if curr_char == '\r' {
            if char_idx + 1 < content.len_chars() && content.char(char_idx + 1) == '\n' {
                 // Skip both \r and \n
                 if char_idx + 2 >= content.len_chars() {
                     return content.len_bytes();
                 }
                 return content.char_to_byte(char_idx + 2);
            }
        }

        if char_idx + 1 >= content.len_chars() {
            return content.len_bytes();
        }
        content.char_to_byte(char_idx + 1)
    }

    fn utf16_index_to_byte_in_str(text: &str, utf16_index: usize) -> usize {
        let mut count = 0;
        for (byte_index, ch) in text.char_indices() {
            let next = count + ch.len_utf16();
            if next > utf16_index {
                return byte_index;
            }
            count = next;
        }
        text.len()
    }

    fn utf16_range_to_byte_range_in_str(
        text: &str,
        range_utf16: &Range<usize>,
    ) -> Range<usize> {
        let start = Self::utf16_index_to_byte_in_str(text, range_utf16.start);
        let end = Self::utf16_index_to_byte_in_str(text, range_utf16.end);
        if start <= end {
            start..end
        } else {
            end..start
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
        highlights: &[(Range<usize>, Hsla)],
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
}

impl EntityInputHandler for CodeEditor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.core.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.core.range_to_utf16(&range));
        Some(self.core.content.byte_slice(range).to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        // Return primary selection for IME
        let primary = self.core.primary_selection();
        Some(UTF16Selection {
            range: self.core.range_to_utf16(&primary.range()),
            reversed: primary.head < primary.anchor,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.core
            .marked_range
            .as_ref()
            .map(|range| self.core.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.core.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut range = range_utf16
            .as_ref()
            .map(|range_utf16| self.core.range_from_utf16(range_utf16))
            .or(self.core.marked_range.clone())
            .unwrap_or(self.core.primary_selection().range());

        let content_len = self.core.content.len_bytes();
        if range.start > content_len || range.end > content_len {
            range.start = range.start.min(content_len);
            range.end = range.end.min(content_len);
        }

        // Compute incremental range BEFORE applying edit
        let (start_line, start_col) = self.lsp_position_for_index(range.start);
        let (end_line, end_col) = self.lsp_position_for_index(range.end);

        self.core.replace_range(range.clone(), new_text);

        // Incremental analyze to avoid full-document reload
        if let Some(analyzer) = &self.sweetline_analyzer {
            let result = analyzer.analyze_incremental(
                start_line,
                start_col,
                end_line,
                end_col,
                new_text,
            );
            self.update_highlights_from_result(result);
        } else {
            self.sync_sweetline_document(cx);
        }

        self.update_completion(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut range = range_utf16
            .as_ref()
            .map(|range_utf16| self.core.range_from_utf16(range_utf16))
            .or(self.core.marked_range.clone())
            .unwrap_or(self.core.primary_selection().range());

        // Ensure range is within bounds to prevent crash
        let content_len = self.core.content.len_bytes();
        if range.start > content_len || range.end > content_len {
            eprintln!("Warning: replace_and_mark_text_in_range range out of bounds: {:?} (len: {})", range, content_len);
            range.start = range.start.min(content_len);
            range.end = range.end.min(content_len);
        }

        // Compute incremental range BEFORE applying edit
        let (start_line, start_col) = self.lsp_position_for_index(range.start);
        let (end_line, end_col) = self.lsp_position_for_index(range.end);

        self.core.replace_range(range.clone(), new_text);

        if let Some(analyzer) = &self.sweetline_analyzer {
            let result = analyzer.analyze_incremental(
                start_line,
                start_col,
                end_line,
                end_col,
                new_text,
            );
            self.update_highlights_from_result(result);
        } else {
            self.sync_sweetline_document(cx);
        }

        if !new_text.is_empty() {
            let new_end = range.start + new_text.len();
            if new_end > self.core.content.len_bytes() {
                eprintln!("CRITICAL: marked_range out of bounds! Range: {:?}, TextLen: {}, ContentLen: {}", 
                    range, new_text.len(), self.core.content.len_bytes());
            }
            self.core.marked_range = Some(range.start..new_end);
        } else {
            self.core.marked_range = None;
        }

        if let Some(new_range_utf16) = new_selected_range_utf16 {
            let new_range = Self::utf16_range_to_byte_range_in_str(new_text, &new_range_utf16);
            let start = range.start + new_range.start;
            let end = range.start + new_range.end;
            self.core.selections = vec![Selection::new(start, end)];
        } else {
            let new_pos = range.start + new_text.len();
            self.core.selections = vec![Selection::new(new_pos, new_pos)];
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let bounds = self.layout.last_bounds.unwrap_or(bounds);
        let mut range = self.core.range_from_utf16(&range_utf16);
        if range.start > range.end {
            std::mem::swap(&mut range.start, &mut range.end);
        }
        let content = &self.core.content;

        let line_count = content.len_lines().max(1);
        let max_digits = line_count.to_string().len();

        let (line_index, _, line_start) = Self::line_col_for_index(content, range.start);
        let line_height = self.layout.line_height();
        let text_x = self.layout.text_x(bounds, max_digits);
        let y = self.layout.line_y(bounds, line_index);

        let line_slice = content.line(line_index);
        let mut line_text = line_slice.to_string();
        if line_text.ends_with('\n') {
            line_text.pop();
            if line_text.ends_with('\r') {
                line_text.pop();
            }
        }

        let line = self.get_cached_shape_line(
            window,
            &line_text,
            self.layout.font_size,
            line_index,
            line_start,
        );
        let line_len = line_text.len();
        let start_in_line = (range.start - line_start).min(line_len);
        let end_in_line = (range.end - line_start).min(line_len);
        let start_x = line.x_for_index(start_in_line);
        let end_x = line.x_for_index(end_in_line);
        Some(Bounds::from_corners(
            point(text_x + start_x, y),
            point(text_x + end_x, y + line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.layout.last_bounds?;
        let content = &self.core.content;

        let line_count = content.len_lines().max(1);
        let max_digits = line_count.to_string().len();
        let text_x = self.layout.text_x(bounds, max_digits);

        let mut line_index = self.layout.line_index_for_y(bounds, point.y);
        if line_index >= content.len_lines() {
            line_index = content.len_lines().saturating_sub(1);
        }

        let line_start = content.line_to_byte(line_index);
        let line_slice = content.line(line_index);
        let mut line_text = line_slice.to_string();
        if line_text.ends_with('\n') {
            line_text.pop();
            if line_text.ends_with('\r') {
                line_text.pop();
            }
        }

        let line = self.get_cached_shape_line(
            window,
            &line_text,
            self.layout.font_size,
            line_index,
            line_start,
        );
        let local_x = (point.x - text_x).max(px(0.0));
        let utf8_index = Self::clamp_to_char_boundary(
            &line_text,
            line.index_for_x(local_x).unwrap_or(line_text.len()),
        );
        Some(self.core.offset_to_utf16(line_start + utf8_index))
    }
}

impl CodeEditor {
    fn hover_info_at(&self, index: usize) -> Option<(String, DecorationColor)> {
        let mut best: Option<(&Decoration, usize)> = None;
        for d in &self.decorations {
            if d.message.is_none() {
                continue;
            }
            if index < d.range.start || index >= d.range.end {
                continue;
            }
            let len = d.range.end.saturating_sub(d.range.start);
            match best {
                None => best = Some((d, len)),
                Some((prev, prev_len)) => {
                    if len < prev_len
                        || (len == prev_len && d.color.priority() > prev.color.priority())
                    {
                        best = Some((d, len));
                    }
                }
            }
        }
        best.map(|(d, _)| (d.message.clone().unwrap_or_default(), d.color))
            .filter(|(t, _)| !t.is_empty())
    }

    fn update_hover_popup(&mut self, pos: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
        self.process_lsp_messages(cx);

        let index = self.index_for_point(pos, window);
        
        // Check local decorations first
        let next = index
            .and_then(|i| self.hover_info_at(i))
            .map(|(text, color)| HoverPopup {
                text,
                position: pos,
                color,
            });

        if let Some(next_popup) = next {
             self.hover_popup = Some(next_popup);
             cx.notify();
             return;
        }
        
        // If no local decoration, request from LSP
        if let Some(_idx) = index {
            /*
            let (line, col) = self.lsp_position_for_index(idx);
            
            // Debounce/Check duplicate: If we already requested hover for this position recently, skip
            // We use a simplified check here. In a robust impl, we'd want true debouncing.
            let already_pending = self.lsp_manager.pending_requests.values().any(|k| {
                 if let LspRequestKind::Hover { index, .. } = k {
                     index == &idx
                 } else {
                     false
                 }
            });
            */

// LSP hover request removed
        }
        
        // Don't clear immediately if waiting for LSP, but maybe we should?
        // If we moved, the old popup might be stale.
        // Let's keep it until new one arrives or explicitly cleared?
        // Actually, if we moved to a new place without local info, we should probably clear the old one until LSP replies.
        if self.hover_popup.is_some() {
             self.hover_popup = None;
             cx.notify();
        }
    }

    fn index_for_point(&self, point: Point<Pixels>, window: &Window) -> Option<usize> {
        let bounds = self.layout.last_bounds?;
        let content = &self.core.content;

        let line_count = content.len_lines().max(1);
        let max_digits = line_count.to_string().len();

        let text_x = self.layout.text_x(bounds, max_digits);

        let mut line_index = self.layout.line_index_for_y(bounds, point.y);

        if line_index >= line_count {
            line_index = line_count.saturating_sub(1);
        }

        let line_start = content.line_to_byte(line_index);
        let line_slice = content.line(line_index);
        let mut line_text_string = line_slice.to_string();
        if line_text_string.ends_with('\n') {
            line_text_string.pop();
            if line_text_string.ends_with('\r') {
                line_text_string.pop();
            }
        }
        let line_text = &line_text_string;

        let line = self.get_cached_shape_line(
            window,
            line_text,
            self.layout.font_size,
            line_index,
            line_start,
        );
        let local_x = (point.x - text_x).max(px(0.0));
        let utf8_index = Self::clamp_to_char_boundary(
            line_text,
            line.index_for_x(local_x).unwrap_or(line_text.len()),
        );
        let index = line_start + utf8_index;
        if index > content.len_bytes() {
            Some(content.len_bytes())
        } else {
            Some(index)
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Check scrollbar
        if let Some(bounds) = self.layout.last_bounds {
            let content_height = self.layout.content_height(self.core.content.len_lines());
            let thumb_bounds = self.layout.thumb_bounds(bounds, content_height);

            let scrollbar_width = self.layout.scrollbar_width();
            let track_bounds = Bounds::new(
                point(bounds.right() - scrollbar_width, bounds.top()),
                size(scrollbar_width, bounds.size.height),
            );

            if track_bounds.contains(&event.position) {
                if thumb_bounds.contains(&event.position) {
                    self.dragging_scrollbar = true;
                    self.drag_start_y = Some(event.position.y);
                    self.scroll_start_y = Some(self.layout.scroll_offset.y);
                } else {
                    let percent = (event.position.y - bounds.top()) / bounds.size.height;
                    let max_scroll = (content_height - bounds.size.height).max(px(0.0));
                    self.layout.scroll_offset.y = -max_scroll * percent;
                }
                cx.notify();
                return;
            }
        }

        self.hover_popup = None;

        if let Some(index) = self.index_for_point(event.position, window) {
            if event.modifiers.alt {
                // Add cursor
                self.core.add_cursor(index);
                cx.notify();
            } else if event.modifiers.shift {
                // Extend last selection
                self.select_to(index, cx);
            } else {
                // Reset to single cursor
                self.set_cursor(index, cx);
            }
        }
    }

    fn on_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.dragging_scrollbar = false;
        self.drag_start_y = None;
        self.scroll_start_y = None;
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dragging_scrollbar {
            if event.pressed_button != Some(MouseButton::Left) {
                self.dragging_scrollbar = false;
                return;
            }

            if let (Some(start_y), Some(scroll_start), Some(bounds)) = (
                self.drag_start_y,
                self.scroll_start_y,
                self.layout.last_bounds,
            ) {
                let delta_y = event.position.y - start_y;
                let content_height = self.layout.content_height(self.core.content.len_lines());
                let view_height = bounds.size.height;

                let thumb_bounds = self.layout.thumb_bounds(bounds, content_height);
                let track_height = view_height;
                let thumb_travel_range = track_height - thumb_bounds.size.height;

                if thumb_travel_range > px(0.0) {
                    let scroll_range = content_height - view_height;
                    let scroll_ratio = scroll_range / thumb_travel_range;
                    let scroll_delta = -delta_y * scroll_ratio;

                    let new_scroll_y = (scroll_start + scroll_delta).clamp(-scroll_range, px(0.0));
                    self.layout.scroll_offset.y = new_scroll_y;
                    cx.notify();
                }
            }
            return;
        }

        if event.pressed_button.is_none() {
            self.update_hover_popup(event.position, window, cx);
            return;
        }
        if let Some(index) = self.index_for_point(event.position, window) {
            self.select_to(index, cx);
        }
    }
}

impl Render for CodeEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = cx.entity();
        let focus_handle = self.focus_handle.clone();

        div()
            .size_full()
            .key_context("CodeEditor")
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_modifiers_changed(cx.listener(Self::on_modifiers_changed))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_line))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::toggle_find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_prev))
            .on_action(cx.listener(Self::cancel_find))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::go_to_definition))
            .on_action(cx.listener(Self::signature_help))
            .on_action(cx.listener(Self::format_document))
            .child(code_editor_canvas(editor, focus_handle))
    }
}

pub fn code_editor_canvas(
    editor: Entity<CodeEditor>,
    focus_handle: FocusHandle,
) -> impl IntoElement {
    canvas(
        |bounds, _window, _cx| bounds,
        move |bounds, _layout, window, cx| {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, editor.clone()),
                cx,
            );
            editor.update(cx, |editor, _cx| {
                editor.layout.last_bounds = Some(bounds);
            });

            let (
                layout,
                content,
                selections,
                completion_active,
                completion_items,
                completion_index,
                decorations,
                hover_popup,
                git_diff_map,
            ) = {
                let state = editor.read(cx);
                (
                    state.layout,
                    state.core.content.clone(),
                    state.core.selections.clone(),
                    state.core.completion_active,
                    state.core.completion_items.clone(),
                    state.core.completion_index,
                    state.decorations.clone(),
                    state.hover_popup.clone(),
                    state.git_diff_map.clone(),
                )
            };

            let font_size = layout.font_size;
            let line_height = layout.line_height();

            let line_count = content.len_lines().max(1);
            let max_digits = line_count.to_string().len();

            let text_x = layout.text_x(bounds, max_digits);

            let primary = selections.last().cloned().unwrap_or(Selection::new(0, 0));
            let primary_head = primary.head;

            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                let (current_line, _, _) = CodeEditor::line_col_for_index(&content, primary_head);

                let gutter_width = layout.gutter_width(max_digits);
                let text_area_bounds = Bounds::from_corners(
                    point(bounds.left() + gutter_width, bounds.top()),
                    bounds.bottom_right(),
                );

                let start_line = layout.line_index_for_y(bounds, bounds.top());
                let end_line =
                    (layout.line_index_for_y(bounds, bounds.bottom()) + 1).min(line_count);

                // 1. Draw Global Backgrounds (Current Line Highlight and Git Diff Backgrounds)
                for i in start_line..end_line {
                    let y = layout.line_y(bounds, i);
                    
                    // Current Line Highlight
                    if i == current_line {
                        let highlight_bounds = Bounds::from_corners(
                            point(bounds.left(), y),
                            point(bounds.right(), y + line_height),
                        );
                        window.paint_quad(fill(highlight_bounds, rgba(0xffffff0d)));
                    }

                    // Git Diff Background Highlight
                    if let Some(status) = git_diff_map.get(&i) {
                         let bg_color = match status {
                             GitDiffStatus::Added => Some(rgba(0x2ea04333)), // Green with ~20% opacity
                             GitDiffStatus::Modified => Some(rgba(0x005cc533)), // Blue with ~20% opacity
                             GitDiffStatus::Deleted => None, // Do not highlight background for deletions (since text is gone)
                         };
                         
                         if let Some(color) = bg_color {
                             let highlight_bounds = Bounds::from_corners(
                                 point(bounds.left(), y),
                                 point(bounds.right(), y + line_height),
                             );
                             window.paint_quad(fill(highlight_bounds, color));
                         }
                    }
                }

                // 2. Draw Gutter
                for i in start_line..end_line {
                    let y = layout.line_y(bounds, i);

                    if let Some(status) = git_diff_map.get(&i) {
                         let color = match status {
                             GitDiffStatus::Added => rgb(0x2ea043),
                             GitDiffStatus::Modified => rgb(0x005cc5),
                             GitDiffStatus::Deleted => rgb(0xd73a49),
                         };
                         let indicator_bounds = Bounds::from_corners(
                             point(bounds.left() + px(2.0), y),
                             point(bounds.left() + px(6.0), y + line_height),
                         );
                         window.paint_quad(fill(indicator_bounds, color));
                    }

                    let number_text = format!("{}", i + 1);
                    let number_line = CodeEditor::shape_line(
                        window,
                        &number_text,
                        rgb(0xff8b949e).into(),
                        font_size,
                    );

                    let number_width = number_line.width;
                    let number_x = bounds.left() + gutter_width - px(8.0) - number_width;

                    number_line
                        .paint(point(number_x, y), line_height, window, cx)
                        .ok();
                    
                    // Draw Diff Symbols (+/~)
                    if let Some(status) = git_diff_map.get(&i) {
                         let (symbol, color) = match status {
                             GitDiffStatus::Added => (Some("+"), rgb(0x2ea043)),
                             GitDiffStatus::Modified => (Some("~"), rgb(0x005cc5)),
                             GitDiffStatus::Deleted => (None, rgb(0xd73a49)),
                         };
                         
                         if let Some(sym) = symbol {
                             let symbol_line = CodeEditor::shape_line(
                                 window,
                                 sym,
                                 color.into(),
                                 font_size * 0.8, // Slightly smaller
                             );
                             // Position symbol between left edge and line number
                             // Or maybe just replace line number? Usually it's next to it.
                             // Let's put it at left edge + padding
                             let symbol_x = bounds.left() + px(10.0);
                             symbol_line
                                 .paint(point(symbol_x, y + px(1.0)), line_height, window, cx)
                                 .ok();
                         }
                    }
                }

                // Check for diff marker at end of file (deleted content after last line)
                let last_line_idx = content.len_lines();
                if end_line == last_line_idx {
                    if let Some(status) = git_diff_map.get(&last_line_idx) {
                         let color = match status {
                             GitDiffStatus::Added => rgb(0x2ea043),
                             GitDiffStatus::Modified => rgb(0x005cc5),
                             GitDiffStatus::Deleted => rgb(0xd73a49),
                         };
                         let y = layout.line_y(bounds, last_line_idx);
                         let indicator_bounds = Bounds::from_corners(
                             point(bounds.left() + px(2.0), y),
                             point(bounds.left() + px(6.0), y + line_height),
                         );
                         window.paint_quad(fill(indicator_bounds, color));
                    }
                }

                // 3. Draw Text Area
                window.with_content_mask(
                    Some(ContentMask {
                        bounds: text_area_bounds,
                    }),
                    |window| {
                        for i in start_line..end_line {
                            let line_start = content.line_to_byte(i);
                            let line_slice = content.line(i);
                            let mut line_text_string = line_slice.to_string();
                            if line_text_string.ends_with('\n') {
                                line_text_string.pop();
                                if line_text_string.ends_with('\r') {
                                    line_text_string.pop();
                                }
                            }
                            let line_text = &line_text_string;
                            let y = layout.line_y(bounds, i);

                            // Draw Selection Backgrounds
                            for selection in &selections {
                                if !selection.is_empty() {
                                    let line_start = content.line_to_byte(i);
                                    let line_end_incl_newline = line_start + line_slice.len_bytes();

                                    let sel_range = selection.range();
                                    let sel_start = sel_range.start.max(line_start);
                                    let sel_end = sel_range.end.min(line_end_incl_newline);

                                    if sel_start < sel_end {
                                        let start_in_line = sel_start - line_start;
                                        let end_in_line = sel_end - line_start;
                                        let line_len = line_text.len();

                                        let shape_start = CodeEditor::clamp_to_char_boundary(
                                            line_text,
                                            start_in_line.min(line_len),
                                        );
                                        let shape_end = CodeEditor::clamp_to_char_boundary(
                                            line_text,
                                            end_in_line.min(line_len),
                                        );

                                        let text_line_shape =
                                            editor.read(cx).get_cached_shape_line(
                                                window, line_text, font_size, i, line_start,
                                            );
                                        let start_x = text_line_shape.x_for_index(shape_start);
                                        let mut end_x = text_line_shape.x_for_index(shape_end);

                                        if end_in_line > line_len {
                                            end_x += px(10.0);
                                        }

                                        let rect_bounds = Bounds::from_corners(
                                            point(text_x + start_x, y),
                                            point(text_x + end_x, y + line_height),
                                        );
                                        window.paint_quad(fill(rect_bounds, rgba(0x264f78aa)));
                                    }
                                }
                            }

                            // Draw Text
                            let text_line = editor
                                .read(cx)
                                .get_cached_shape_line(window, line_text, font_size, i, line_start);
                            text_line
                                .paint(point(text_x, y), line_height, window, cx)
                                .ok();

                            for d in &decorations {
                                let line_end_incl_newline = line_start + line_slice.len_bytes();
                                let deco_start = d.range.start.max(line_start);
                                let deco_end = d.range.end.min(line_end_incl_newline);
                                if deco_start >= deco_end {
                                    continue;
                                }

                                let line_len = line_text.len();
                                let start_in_line = (deco_start - line_start).min(line_len);
                                let end_in_line = (deco_end - line_start).min(line_len);
                                if start_in_line >= end_in_line {
                                    continue;
                                }

                                let shape_start =
                                    CodeEditor::clamp_to_char_boundary(line_text, start_in_line);
                                let shape_end =
                                    CodeEditor::clamp_to_char_boundary(line_text, end_in_line);
                                let start_x = text_line.x_for_index(shape_start);
                                let end_x = text_line.x_for_index(shape_end);
                                let underline_y = y + line_height - px(4.0);
                                CodeEditor::paint_squiggly(
                                    window,
                                    text_x + start_x,
                                    text_x + end_x,
                                    underline_y,
                                    line_height,
                                    d.color.rgba(),
                                );
                            }
                        }

                        // Draw Cursors
                        for selection in &selections {
                            let head = selection.head;
                            let is_primary = selection.anchor == primary.anchor && selection.head == primary.head;
                            let (line, _, line_start) =
                                CodeEditor::line_col_for_index(&content, head);

                            if line >= start_line && line < end_line {
                                let line_slice = content.line(line);
                                let mut line_text_string = line_slice.to_string();
                                if line_text_string.ends_with('\n') {
                                    line_text_string.pop();
                                    if line_text_string.ends_with('\r') {
                                        line_text_string.pop();
                                    }
                                }
                                let line_text = &line_text_string;

                                let line_shape = editor.read(cx).get_cached_shape_line(
                                    window, line_text, font_size, line, line_start,
                                );
                                let local_index = head
                                    .saturating_sub(line_start)
                                    .min(line_text.len());
                                let cursor_x = text_x + line_shape.x_for_index(local_index);
                                let cursor_y = layout.line_y(bounds, line);
                                let caret_width = if is_primary { px(2.0) } else { px(1.0) };
                                let caret_height = (line_height - px(2.0)).max(px(0.0));
                                let cursor_bounds = Bounds::new(
                                    point(cursor_x, cursor_y + px(1.0)),
                                    size(caret_width, caret_height),
                                );
                                let color = if is_primary {
                                    rgb(0xffffffff)
                                } else {
                                    rgba(0xffffffb3)
                                };
                                window.paint_quad(fill(cursor_bounds, color));
                            }
                        }

                        // Draw Completion Menu (Primary cursor only)
                        if completion_active && !completion_items.is_empty() {
                            let (line, _, line_start) =
                                CodeEditor::line_col_for_index(&content, primary_head);
                            let line_slice = content.line(line);
                            let mut line_text_string = line_slice.to_string();
                            if line_text_string.ends_with('\n') {
                                line_text_string.pop();
                                if line_text_string.ends_with('\r') {
                                    line_text_string.pop();
                                }
                            }
                            let line_text = &line_text_string;

                            let line_shape = editor.read(cx).get_cached_shape_line(
                                window, line_text, font_size, line, line_start,
                            );
                            let local_index = primary_head
                                .saturating_sub(line_start)
                                .min(line_text.len());
                            let cursor_x = text_x + line_shape.x_for_index(local_index);
                            let cursor_y = layout.line_y(bounds, line);

                            let menu_x = cursor_x;
                            let menu_y = cursor_y + line_height + px(4.0); // Add a little gap

                            let item_height = line_height;
                            let menu_width = px(250.0);
                            
                            // Calculate visible range
                            let max_visible_items = 10;
                            let total_items = completion_items.len();
                            let visible_count = total_items.min(max_visible_items);
                            let menu_height = item_height * visible_count as f32;
                            
                            let scroll_index = editor.read(cx).completion_scroll_offset as usize;
                            let visible_items = &completion_items[scroll_index..(scroll_index + visible_count).min(total_items)];

                            let menu_bounds =
                                Bounds::new(point(menu_x, menu_y), size(menu_width, menu_height));

                            // Paint shadow
                            CodeEditor::paint_soft_shadow(window, menu_bounds, px(4.0));

                            let mut menu_quad = fill(menu_bounds, rgb(0x252526));
                            menu_quad.border_widths = Edges::all(px(1.0));
                            menu_quad.border_color = rgb(0x454545).into();
                            menu_quad.corner_radii = Corners::all(px(4.0));
                            window.paint_quad(menu_quad);
                            
                            // Clip content to menu bounds
                            window.with_content_mask(Some(ContentMask { bounds: menu_bounds }), |window| {
                                for (i, item) in visible_items.iter().enumerate() {
                                    let global_index = scroll_index + i;
                                    let item_y = menu_y + item_height * i as f32;
                                    let item_bounds = Bounds::new(
                                        point(menu_x, item_y),
                                        size(menu_width, item_height),
                                    );

                                    if global_index == completion_index {
                                        window.paint_quad(fill(item_bounds, rgb(0x04395e)));
                                    }

                                    // Icon
                                    let icon_text = item.kind.icon_text();
                                    let icon_color = item.kind.color();
                                    let icon_line = CodeEditor::shape_line(
                                        window, icon_text, icon_color, font_size,
                                    );
                                    let icon_height = icon_line.ascent + icon_line.descent;
                                    let icon_y = item_y + (item_height - icon_height) / 2.0;
                                    icon_line
                                        .paint(point(menu_x + px(6.0), icon_y), item_height, window, cx)
                                        .ok();

                                    // Label
                                    let label_line = CodeEditor::shape_line(
                                        window,
                                        &item.label,
                                        rgb(0xcccccc).into(),
                                        font_size,
                                    );
                                    let label_height = label_line.ascent + label_line.descent;
                                    let label_y = item_y + (item_height - label_height) / 2.0;
                                    label_line
                                        .paint(
                                            point(menu_x + px(30.0), label_y),
                                            item_height,
                                            window,
                                            cx,
                                        )
                                        .ok();

                                    // Detail
                                    if !item.detail.is_empty() {
                                        let detail_line = CodeEditor::shape_line(
                                            window,
                                            &item.detail,
                                            rgb(0x808080).into(),
                                            font_size,
                                        );
                                        let detail_height = detail_line.ascent + detail_line.descent;
                                        let detail_y = item_y + (item_height - detail_height) / 2.0;
                                        let detail_x =
                                            menu_x + menu_width - detail_line.width - px(8.0);
                                        
                                        // Only draw if it fits
                                        if detail_x > menu_x + px(30.0) + label_line.width + px(10.0) {
                                            detail_line
                                                .paint(point(detail_x, detail_y), item_height, window, cx)
                                                .ok();
                                        }
                                    }
                                }
                            });
                            
                            // Scrollbar for completion menu
                            if total_items > max_visible_items {
                                let scrollbar_width = px(4.0);
                                let track_bounds = Bounds::new(
                                    point(menu_bounds.right() - scrollbar_width, menu_bounds.top()),
                                    size(scrollbar_width, menu_bounds.size.height),
                                );
                                
                                let thumb_height = menu_bounds.size.height * (visible_count as f32 / total_items as f32);
                                let thumb_y = menu_bounds.top() + (menu_bounds.size.height - thumb_height) * (scroll_index as f32 / (total_items - visible_count) as f32);
                                
                                let thumb_bounds = Bounds::new(
                                    point(menu_bounds.right() - scrollbar_width, thumb_y),
                                    size(scrollbar_width, thumb_height)
                                );
                                
                                window.paint_quad(fill(track_bounds, rgba(0x00000000)));
                                window.paint_quad(fill(thumb_bounds, rgba(0x80808080)));
                            }
                        }

                        // Draw Scrollbar
                        let content_height = layout.content_height(line_count);
                        let thumb_bounds = layout.thumb_bounds(bounds, content_height);

                        if !thumb_bounds.is_empty() {
                            let scrollbar_width = layout.scrollbar_width();
                            let track_bounds = Bounds::new(
                                point(bounds.right() - scrollbar_width, bounds.top()),
                                size(scrollbar_width, bounds.size.height),
                            );

                            window.paint_quad(fill(track_bounds, rgba(0x00000000)));

                            let mut thumb_quad = fill(thumb_bounds, rgba(0x42424280));
                            thumb_quad.corner_radii = Corners::all(px(4.0));
                            window.paint_quad(thumb_quad);
                        }
                    },
                );

                if let Some(hover) = &hover_popup {
                    let popup_font = font_size.min(px(14.0));
                    let popup_line_height = popup_font * 1.4;
                    let text_line =
                        CodeEditor::shape_line(window, &hover.text, rgb(0xffffffff).into(), popup_font);

                    let padding_x = px(10.0);
                    let padding_y = px(6.0);
                    let popup_w = (text_line.width + padding_x * 2.0).max(px(60.0));
                    let popup_h = popup_line_height + padding_y * 2.0;

                    let mut x = hover.position.x + px(12.0);
                    let mut y = hover.position.y + px(18.0);
                    x = x
                        .min(bounds.right() - popup_w - px(4.0))
                        .max(bounds.left() + px(4.0));
                    y = y
                        .min(bounds.bottom() - popup_h - px(4.0))
                        .max(bounds.top() + px(4.0));

                    let popup_bounds = Bounds::new(point(x, y), size(popup_w, popup_h));
                    
                    // Paint shadow
                    CodeEditor::paint_soft_shadow(window, popup_bounds, px(4.0));
                    
                    let mut popup_quad = fill(popup_bounds, rgba(0x1e1e1ef0));
                    popup_quad.border_widths = Edges::all(px(1.0));
                    popup_quad.border_color = hover.color.rgba().into();
                    window.paint_quad(popup_quad);
                    text_line
                        .paint(
                            point(x + padding_x, y + padding_y),
                            popup_line_height,
                            window,
                            cx,
                        )
                        .ok();
                }
            });
        },
    )
    .size_full()
}
