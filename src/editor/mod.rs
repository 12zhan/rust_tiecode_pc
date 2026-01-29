use gpui::*;
use ropey::Rope;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::cmp::Ordering;

pub mod core;
pub mod layout;
pub mod completion;
pub mod undo;

use self::core::{EditorCore, Selection};
use self::layout::EditorLayout;
use self::completion::{CompletionItem, CompletionKind, CPP_KEYWORDS};

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
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        ToggleFind,
        FindNext,
        FindPrev,
        CancelFind
    ]
);

pub struct CodeEditor {
    pub focus_handle: FocusHandle,
    pub core: EditorCore,
    pub layout: EditorLayout,
    render_cache: Arc<Mutex<LruCache<String, ShapedLine>>>,
    dragging_scrollbar: bool,
    drag_start_y: Option<Pixels>,
    scroll_start_y: Option<Pixels>,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            core: EditorCore::new(),
            layout: EditorLayout::new(),
            render_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))),
            dragging_scrollbar: false,
            drag_start_y: None,
            scroll_start_y: None,
        }
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.core.content = Rope::from(content);
        self.core.set_cursor(0);
        cx.notify();
    }

    pub fn set_cursor(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.set_cursor(index);
        cx.notify();
    }

    pub fn select_to(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.select_to(index);
        cx.notify();
    }

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.core.insert_text(text);
        self.update_completion(cx);
        cx.notify();
    }

    pub fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.core.delete_range(range);
        self.update_completion(cx);
        cx.notify();
    }

    fn update_completion(&mut self, cx: &mut Context<Self>) {
        let primary = self.core.primary_selection();
        if primary.is_empty() {
            let cursor = primary.head;
            let content = &self.core.content;
            
            let mut word_start = cursor;
            let mut current_idx = cursor;
            
            while current_idx > 0 {
                let char_idx = content.byte_to_char(current_idx);
                if char_idx == 0 && current_idx > 0 { break; }
                let prev_char_idx = char_idx - 1;
                let ch = content.char(prev_char_idx);
                let char_len = ch.len_utf8();
                let prev_byte_idx = current_idx - char_len;
                
                if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                    word_start = current_idx;
                    break;
                }
                current_idx = prev_byte_idx;
                word_start = current_idx;
            }
            
            if word_start < cursor {
                let prefix_string = content.byte_slice(word_start..cursor).to_string();
                let prefix = prefix_string.as_str();
                
                if !prefix.is_empty() {
                    let mut items = Vec::new();
                    
                    // Add mock data
                    let mock_data = vec![
                        CompletionItem { label: "main".to_string(), kind: CompletionKind::Function, detail: " void".to_string() },
                        CompletionItem { label: "miss".to_string(), kind: CompletionKind::Class, detail: " class".to_string() },
                        CompletionItem { label: "miii".to_string(), kind: CompletionKind::Text, detail: " text".to_string() },
                        CompletionItem { label: "min".to_string(), kind: CompletionKind::Variable, detail: " int".to_string() },
                        CompletionItem { label: "ant".to_string(), kind: CompletionKind::Variable, detail: " int".to_string() },
                        CompletionItem { label: "Demo".to_string(), kind: CompletionKind::Class, detail: "".to_string() },
                    ];
                    for item in mock_data {
                        if item.label.starts_with(prefix) && item.label != prefix {
                            items.push(item);
                        }
                    }
                    
                    // Add CPP keywords
                    for keyword in CPP_KEYWORDS {
                        if keyword.starts_with(prefix) && *keyword != prefix {
                             items.push(CompletionItem {
                                 label: keyword.to_string(),
                                 kind: CompletionKind::Keyword,
                                 detail: "".to_string(),
                             });
                        }
                    }
                    
                    if !items.is_empty() {
                        self.core.completion_active = true;
                        self.core.completion_items = items;
                        self.core.completion_index = 0;
                        cx.notify();
                        return;
                    }
                }
            }
        }
        
        if self.core.completion_active {
            self.core.completion_active = false;
            self.core.completion_items.clear();
            self.core.completion_index = 0;
            cx.notify();
        }
    }

    fn confirm_completion(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = self.core.completion_items.get(self.core.completion_index) {
            let label = item.label.clone();
            
            let primary = self.core.primary_selection();
            let cursor = primary.head;
            let content = &self.core.content;
            let mut word_start = cursor;
            
            let mut current_idx = cursor;
            while current_idx > 0 {
                let char_idx = content.byte_to_char(current_idx);
                if char_idx == 0 && current_idx > 0 {
                     break; 
                }
                let prev_char_idx = char_idx - 1;
                let ch = content.char(prev_char_idx);
                let char_len = ch.len_utf8();
                let prev_byte_idx = current_idx - char_len;
                
                if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                    word_start = current_idx;
                    break;
                }
                current_idx = prev_byte_idx;
                word_start = current_idx;
            }
            
            self.core.replace_range(word_start..cursor, &label);
            
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
        
        self.core.selections = ranges_to_delete.into_iter()
            .map(|r| Selection::new(r.start, r.end))
            .collect();
        self.core.merge_selections(); // Merge overlapping lines
        
        self.core.delete_selection();
        cx.notify();
    }

    fn enter(&mut self, _: &Enter, _window: &mut Window, cx: &mut Context<Self>) {
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

    fn ctrl_shift_tab(&mut self, _: &CtrlShiftTab, _window: &mut Window, _cx: &mut Context<Self>) {
        println!("special: ctrl-shift-tab");
    }

    fn undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        self.core.undo();
        cx.notify();
    }

    fn redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        self.core.redo();
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

    fn move_up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            if self.core.completion_index > 0 {
                self.core.completion_index -= 1;
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
             let cursor = if shift { head } else {
                 if !selection.is_empty() { selection.range().start } else { head }
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
             let cursor = if shift { head } else {
                 if !selection.is_empty() { selection.range().end } else { head }
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
            let delta = event.delta.pixel_delta(px(10.0)).y;
            self.layout.zoom(delta);
            if let Ok(mut cache) = self.render_cache.lock() {
                cache.clear();
            }
        } else {
            let delta = event.delta.pixel_delta(px(20.0));
            
            let bounds = self.layout.last_bounds.unwrap_or_default();
            let view_size = bounds.size;
            
            let line_count = self.core.content.len_lines().max(1);
            let total_height = self.layout.line_height() * line_count as f32;
            let max_scroll_y = (total_height - view_size.height + self.layout.line_height()).max(px(0.0));
            
            let max_line_len = self.core.content.lines().map(|l| l.len_bytes()).max().unwrap_or(0);
            let max_digits = line_count.to_string().len();
            let gutter_width = self.layout.gutter_width(max_digits);
            let char_width = self.layout.font_size * 0.75; 
            let content_width = gutter_width + px(40.0) + (max_line_len as f32 * char_width);
            let max_scroll_x = (content_width - view_size.width).max(px(0.0));

            self.layout.scroll(delta, point(max_scroll_x, max_scroll_y));
        }
        cx.notify();
    }

    fn get_cached_shape_line(
        &self,
        window: &Window,
        text: &str,
        font_size: Pixels,
    ) -> ShapedLine {
        let key = text.to_string();
        if let Ok(mut cache) = self.render_cache.lock() {
            if let Some(line) = cache.get(&key) {
                return line.clone();
            }
        }
        
        let line = Self::shape_code_line(window, text, font_size);
        
        if let Ok(mut cache) = self.render_cache.lock() {
            cache.put(key, line.clone());
        }
        line
    }

    // Helper functions
    fn line_col_for_index(content: &Rope, index: usize) -> (usize, usize, usize) {
        let line_index = content.byte_to_line(index);
        let line_start = content.line_to_byte(line_index);
        let col = index - line_start;
        (line_index, col, line_start)
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
        let char_idx = content.byte_to_char(index);
        if char_idx == 0 {
            return 0;
        }
        content.char_to_byte(char_idx - 1)
    }

    fn next_char_index(content: &Rope, index: usize) -> usize {
        if index >= content.len_bytes() {
            return content.len_bytes();
        }
        let char_idx = content.byte_to_char(index);
        if char_idx + 1 >= content.len_chars() {
            return content.len_bytes();
        }
        content.char_to_byte(char_idx + 1)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let char_idx = self.core.content.byte_to_char(offset);
        self.core.content.slice(0..char_idx).len_utf16_cu()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        let start = self.offset_to_utf16(range.start);
        let end = self.offset_to_utf16(range.end);
        start..end
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        let start_char = self.core.content.utf16_cu_to_char(range_utf16.start);
        let end_char = self.core.content.utf16_cu_to_char(range_utf16.end);
        let start_byte = self.core.content.char_to_byte(start_char);
        let end_byte = self.core.content.char_to_byte(end_char);
        start_byte..end_byte
    }

    fn shape_line(
        window: &Window,
        text: &str,
        color: Hsla,
        font_size: Pixels,
    ) -> ShapedLine {
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

    fn highlight_cpp(text: &str) -> Vec<(Range<usize>, Hsla)> {
        let mut highlights = Vec::new();
        let mut start = 0;
        
        for (i, c) in text.char_indices() {
            if !c.is_alphanumeric() && c != '_' && c != '#' {
                if start < i {
                    let word = &text[start..i];
                    let color = match word {
                        "int" | "char" | "float" | "double" | "bool" | "void" | "long" | "short" | "signed" | "unsigned" => Some(rgb(0x569cd6)),
                        "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" | "break" | "continue" | "return" | "goto" => Some(rgb(0xc586c0)),
                        "struct" | "class" | "enum" | "union" | "typedef" | "typename" | "template" | "namespace" | "using" => Some(rgb(0x569cd6)),
                        "public" | "private" | "protected" | "virtual" | "override" | "static" | "const" | "inline" | "friend" => Some(rgb(0x569cd6)),
                        "true" | "false" | "nullptr" | "this" | "new" | "delete" | "sizeof" | "operator" | "explicit" | "noexcept" => Some(rgb(0x569cd6)),
                        "#include" | "#define" | "#ifdef" | "#ifndef" | "#endif" | "#pragma" => Some(rgb(0xc586c0)),
                        _ => None,
                    };
                    if let Some(c) = color {
                        highlights.push((start..i, c.into()));
                    }
                }
                start = i + c.len_utf8();
            }
        }
        if start < text.len() {
             let word = &text[start..];
             let color = match word {
                "int" | "char" | "float" | "double" | "bool" | "void" | "long" | "short" | "signed" | "unsigned" => Some(rgb(0x569cd6)),
                "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" | "break" | "continue" | "return" | "goto" => Some(rgb(0xc586c0)),
                "struct" | "class" | "enum" | "union" | "typedef" | "typename" | "template" | "namespace" | "using" => Some(rgb(0x569cd6)),
                "public" | "private" | "protected" | "virtual" | "override" | "static" | "const" | "inline" | "friend" => Some(rgb(0x569cd6)),
                "true" | "false" | "nullptr" | "this" | "new" | "delete" | "sizeof" | "operator" | "explicit" | "noexcept" => Some(rgb(0x569cd6)),
                "#include" | "#define" | "#ifdef" | "#ifndef" | "#endif" | "#pragma" => Some(rgb(0xc586c0)),
                _ => None,
            };
            if let Some(c) = color {
                highlights.push((start..text.len(), c.into()));
            }
        }
        highlights
    }

    fn shape_code_line(
        window: &Window,
        text: &str,
        font_size: Pixels,
    ) -> ShapedLine {
        let mut runs = Vec::new();
        let mut last_end = 0;
        let style = window.text_style();
        
        let highlights = Self::highlight_cpp(text);
        
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
                color,
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
        let range = self.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.range_to_utf16(&range));
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
            range: self.range_to_utf16(&primary.range()),
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
            .map(|range| self.range_to_utf16(range))
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
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.core.marked_range.clone())
            .unwrap_or(self.core.primary_selection().range());

        self.core.replace_range(range, new_text);
        
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
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.core.marked_range.clone())
            .unwrap_or(self.core.primary_selection().range());

        self.core.replace_range(range.clone(), new_text);

        if !new_text.is_empty() {
            self.core.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.core.marked_range = None;
        }
        
        if let Some(new_range_utf16) = new_selected_range_utf16 {
             let new_range = self.range_from_utf16(&new_range_utf16);
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
        let range = self.range_from_utf16(&range_utf16);
        let content = &self.core.content;
        
        let line_count = content.len_lines().max(1);
        let max_digits = line_count.to_string().len();

        let (line_index, _, line_start) = Self::line_col_for_index(content, range.start);
        let line_height = self.layout.line_height();
        let text_x = self.layout.text_x(bounds, max_digits);
        let y = self.layout.line_y(bounds, line_index);
        
        let line_slice = content.line(line_index);
        let line_text = line_slice.to_string();
        
        let line = self.get_cached_shape_line(window, &line_text, self.layout.font_size);
        let start_x = line.x_for_index(range.start - line_start);
        let end_x = line.x_for_index(range.end - line_start);
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
        let line_text = line_slice.to_string();
        
        let line = self.get_cached_shape_line(window, &line_text, self.layout.font_size);
        let local_x = point.x - text_x;
        let utf8_index = line.index_for_x(local_x).unwrap_or(line_text.len());
        Some(self.offset_to_utf16(line_start + utf8_index))
    }
}

impl CodeEditor {
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

        let line = self.get_cached_shape_line(window, line_text, self.layout.font_size);
        let local_x = point.x - text_x;
        let utf8_index = line.index_for_x(local_x).unwrap_or(line_text.len());
        Some(line_start + utf8_index)
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
                 size(scrollbar_width, bounds.size.height)
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

        if let Some(index) = self.index_for_point(event.position, window) {
            if event.modifiers.alt {
                // Add cursor
                self.core.add_cursor(index);
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
             
             if let (Some(start_y), Some(scroll_start), Some(bounds)) = (self.drag_start_y, self.scroll_start_y, self.layout.last_bounds) {
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
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::toggle_find))
            .on_action(cx.listener(Self::find_next))
            .on_action(cx.listener(Self::find_prev))
            .on_action(cx.listener(Self::cancel_find))
            .child(code_editor_canvas(editor, focus_handle))
    }
}

pub fn code_editor_canvas(editor: Entity<CodeEditor>, focus_handle: FocusHandle) -> impl IntoElement {
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
            
            let (layout, content, selections, completion_active, completion_items, completion_index) = {
                let state = editor.read(cx);
                (
                    state.layout, 
                    state.core.content.clone(), 
                    state.core.selections.clone(),
                    state.core.completion_active,
                    state.core.completion_items.clone(),
                    state.core.completion_index,
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
                    bounds.bottom_right()
                );

                let start_line = layout.line_index_for_y(bounds, bounds.top());
                let end_line = (layout.line_index_for_y(bounds, bounds.bottom()) + 1).min(line_count);

                // 1. Draw Global Backgrounds (Current Line Highlight)
                for i in start_line..end_line {
                    let y = layout.line_y(bounds, i);
                    if i == current_line {
                        let highlight_bounds = Bounds::from_corners(
                            point(bounds.left(), y),
                            point(bounds.right(), y + line_height)
                        );
                        window.paint_quad(fill(highlight_bounds, rgba(0xffffff0d)));
                    }
                }

                // 2. Draw Gutter
                for i in start_line..end_line {
                    let y = layout.line_y(bounds, i);
                    let number_text = format!("{}", i + 1);
                    let number_line =
                        CodeEditor::shape_line(window, &number_text, rgb(0xff8b949e).into(), font_size);
                    
                    let number_width = number_line.width;
                    let number_x = bounds.left() + gutter_width - px(8.0) - number_width;

                    number_line
                        .paint(point(number_x, y), line_height, window, cx)
                        .ok();
                }

                // 3. Draw Text Area
                window.with_content_mask(Some(ContentMask { bounds: text_area_bounds }), |window| {
                    for i in start_line..end_line {
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
                                    
                                    let shape_start = start_in_line.min(line_len);
                                    let shape_end = end_in_line.min(line_len);
                                    
                                    let text_line_shape = editor.read(cx).get_cached_shape_line(window, line_text, font_size);
                                    let start_x = text_line_shape.x_for_index(shape_start);
                                    let mut end_x = text_line_shape.x_for_index(shape_end);
                                    
                                    if end_in_line > line_len {
                                        end_x += px(10.0);
                                    }
                                    
                                    let rect_bounds = Bounds::from_corners(
                                        point(text_x + start_x, y),
                                        point(text_x + end_x, y + line_height)
                                    );
                                    window.paint_quad(fill(rect_bounds, rgba(0x264f78aa)));
                                }
                             }
                        }

                        // Draw Text
                        let text_line = editor.read(cx).get_cached_shape_line(window, line_text, font_size);
                        text_line
                            .paint(point(text_x, y), line_height, window, cx)
                            .ok();
                    }

                    // Draw Cursors
                    for selection in &selections {
                         let head = selection.head;
                         let (line, _, line_start) = CodeEditor::line_col_for_index(&content, head);
                         
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
                             
                             let line_shape = editor.read(cx).get_cached_shape_line(window, line_text, font_size);
                             let local_index = head.saturating_sub(line_start);
                             let cursor_x = text_x + line_shape.x_for_index(local_index);
                             let cursor_y = layout.line_y(bounds, line);
                             let cursor_bounds = Bounds::new(point(cursor_x, cursor_y), size(px(1.0), line_height));
                             window.paint_quad(fill(cursor_bounds, rgb(0xffffffff)));
                         }
                    }

                    // Draw Completion Menu (Primary cursor only)
                    if completion_active && !completion_items.is_empty() {
                        let (line, _, line_start) = CodeEditor::line_col_for_index(&content, primary_head);
                        let line_slice = content.line(line);
                        let mut line_text_string = line_slice.to_string();
                        if line_text_string.ends_with('\n') {
                            line_text_string.pop();
                            if line_text_string.ends_with('\r') {
                                line_text_string.pop();
                            }
                        }
                        let line_text = &line_text_string;
                        
                        let line_shape = editor.read(cx).get_cached_shape_line(window, line_text, font_size);
                        let local_index = primary_head.saturating_sub(line_start);
                        let cursor_x = text_x + line_shape.x_for_index(local_index);
                        let cursor_y = layout.line_y(bounds, line);
                        
                        let menu_x = cursor_x;
                        let menu_y = cursor_y + line_height;
                        
                        let item_height = line_height;
                        let menu_width = px(200.0);
                        let menu_height = item_height * completion_items.len() as f32;
                        
                        let menu_bounds = Bounds::new(
                            point(menu_x, menu_y),
                            size(menu_width, menu_height)
                        );
                        
                        let mut menu_quad = fill(menu_bounds, rgb(0x252526));
                        menu_quad.border_widths = Edges::all(px(1.0));
                        menu_quad.border_color = rgb(0x454545).into();
                        window.paint_quad(menu_quad);
                        
                        for (i, item) in completion_items.iter().enumerate() {
                            let item_y = menu_y + item_height * i as f32;
                            let item_bounds = Bounds::new(
                                point(menu_x, item_y),
                                size(menu_width, item_height)
                            );
                            
                            if i == completion_index {
                                window.paint_quad(fill(item_bounds, rgb(0x04395e)));
                            }
                            
                            let _icon_bounds = Bounds::new(
                                point(menu_x + px(4.0), item_y),
                                size(px(20.0), item_height)
                            );
                            let icon_text = item.kind.icon_text();
                            let icon_color = item.kind.color();
                            let icon_line = CodeEditor::shape_line(window, icon_text, icon_color, font_size);
                            let icon_height = icon_line.ascent + icon_line.descent;
                            let icon_y = item_y + (item_height - icon_height) / 2.0;
                            icon_line.paint(point(menu_x + px(6.0), icon_y), item_height, window, cx).ok();

                            let label_line = CodeEditor::shape_line(window, &item.label, rgb(0xcccccc).into(), font_size);
                            let label_height = label_line.ascent + label_line.descent;
                            let label_y = item_y + (item_height - label_height) / 2.0;
                            label_line.paint(point(menu_x + px(30.0), label_y), item_height, window, cx).ok();
                            
                            if !item.detail.is_empty() {
                                let detail_line = CodeEditor::shape_line(window, &item.detail, rgb(0x808080).into(), font_size);
                                let detail_height = detail_line.ascent + detail_line.descent;
                                let detail_y = item_y + (item_height - detail_height) / 2.0;
                                let detail_x = menu_x + menu_width - detail_line.width - px(8.0);
                                detail_line.paint(point(detail_x, detail_y), item_height, window, cx).ok();
                            }
                        }
                    }

                    // Draw Scrollbar
                    let content_height = layout.content_height(line_count);
                    let thumb_bounds = layout.thumb_bounds(bounds, content_height);

                    if !thumb_bounds.is_empty() {
                        let scrollbar_width = layout.scrollbar_width();
                        let track_bounds = Bounds::new(
                            point(bounds.right() - scrollbar_width, bounds.top()),
                            size(scrollbar_width, bounds.size.height)
                        );
                        
                        window.paint_quad(fill(track_bounds, rgba(0x00000000)));
                        
                        let mut thumb_quad = fill(thumb_bounds, rgba(0x42424280));
                        thumb_quad.corner_radii = Corners::all(px(4.0));
                        window.paint_quad(thumb_quad);
                    }
                });
            });
        }
    )
    .size_full()
}
