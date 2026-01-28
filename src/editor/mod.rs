use gpui::*;
use std::ops::Range;

pub mod core;
pub mod layout;
pub mod completion;

use self::core::EditorCore;
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
        Paste
    ]
);

pub struct CodeEditor {
    pub focus_handle: FocusHandle,
    pub core: EditorCore,
    pub layout: EditorLayout,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            core: EditorCore::new(),
            layout: EditorLayout::new(),
        }
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.core.content = content;
        self.core.selected_range = 0..0;
        self.core.selection_anchor = 0;
        self.core.marked_range = None;
        self.core.preferred_column = None;
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
        if self.core.selected_range.is_empty() {
            let cursor = self.core.selected_range.start;
            let content = &self.core.content;
            
            let mut word_start = cursor;
            for (i, ch) in content[..cursor].char_indices().rev() {
                 if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                     word_start = i + ch.len_utf8();
                     break;
                 }
                 word_start = i;
            }
            
            if word_start < cursor {
                let prefix = &content[word_start..cursor];
                // Require prefix to have length > 0 to start completion
                // But wait, the user said "input space and delete" triggers completion.
                // The issue was "completion shows all words".
                // I need to ensure I filter correctly.
                
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
            
            let cursor = self.core.selected_range.start;
            let content = &self.core.content;
            let mut word_start = cursor;
            for (i, ch) in content[..cursor].char_indices().rev() {
                 if !ch.is_alphanumeric() && ch != '_' && ch != '#' {
                     word_start = i + ch.len_utf8();
                     break;
                 }
                 word_start = i;
            }
            
            // Delete the prefix
            self.core.delete_range(word_start..cursor);
            // Insert the completion
            self.core.insert_text(&label);
            
            self.core.completion_active = false;
            self.core.completion_items.clear();
            self.core.completion_index = 0;
            cx.notify();
        }
    }

    fn backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.core.content.to_string();
        if !self.core.selected_range.is_empty() {
            self.delete_range(self.core.selected_range.clone(), cx);
            return;
        }
        let cursor = self.core.selected_range.end;
        if cursor == 0 {
            return;
        }
        let prev = Self::prev_char_index(&content, cursor);
        self.delete_range(prev..cursor, cx);
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.core.content.to_string();
        if !self.core.selected_range.is_empty() {
            self.delete_range(self.core.selected_range.clone(), cx);
            return;
        }
        let cursor = self.core.selected_range.end;
        if cursor >= content.len() {
            return;
        }
        let next = Self::next_char_index(&content, cursor);
        self.delete_range(cursor..next, cx);
    }

    fn delete_line(&mut self, _: &DeleteLine, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.core.content.to_string();
        let cursor = self.core.selected_range.start;
        let (line_idx, _, line_start) = Self::line_col_for_index(&content, cursor);
        let starts = Self::line_start_indices(&content);
        
        let range_end = if line_idx + 1 < starts.len() {
            starts[line_idx + 1]
        } else {
            content.len()
        };
        
        let mut range_start = line_start;
        
        // If last line, try to delete previous newline
        if line_idx + 1 >= starts.len() && range_start > 0 {
             range_start -= 1;
             // Check for \r before \n
             if range_start > 0 && content.as_bytes()[range_start - 1] == b'\r' {
                 range_start -= 1;
             }
        }
        
        let range = range_start..range_end;
        
        if range.is_empty() {
             return;
        }

        self.delete_range(range, cx);
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

    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if self.core.selected_range.is_empty() {
            return;
        }
        let text = self.core.content[self.core.selected_range.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if self.core.selected_range.is_empty() {
            return;
        }
        let text = self.core.content[self.core.selected_range.clone()].to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.delete_range(self.core.selected_range.clone(), cx);
    }

    fn paste(&mut self, _: &Paste, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.insert_text(&text, cx);
            }
        }
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
        let content = self.core.content.to_string();
        if window.modifiers().shift {
            let head = if self.core.selection_anchor == self.core.selected_range.start {
                self.core.selected_range.end
            } else {
                self.core.selected_range.start
            };
            let prev = Self::prev_char_index(&content, head);
            self.select_to(prev, cx);
        } else {
            if !self.core.selected_range.is_empty() {
                self.set_cursor(self.core.selected_range.start, cx);
            } else {
                let prev = Self::prev_char_index(&content, self.core.selected_range.start);
                self.set_cursor(prev, cx);
            }
        }
    }

    fn move_right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.core.content.to_string();
        if window.modifiers().shift {
            let head = if self.core.selection_anchor == self.core.selected_range.start {
                self.core.selected_range.end
            } else {
                self.core.selected_range.start
            };
            let next = Self::next_char_index(&content, head);
            self.select_to(next, cx);
        } else {
            if !self.core.selected_range.is_empty() {
                self.set_cursor(self.core.selected_range.end, cx);
            } else {
                let next = Self::next_char_index(&content, self.core.selected_range.end);
                self.set_cursor(next, cx);
            }
        }
    }

    fn move_up(&mut self, _: &Up, window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            if self.core.completion_index > 0 {
                self.core.completion_index -= 1;
                cx.notify();
            }
            return;
        }

        let content = self.core.content.to_string();
        let head = if self.core.selection_anchor == self.core.selected_range.start {
            self.core.selected_range.end
        } else {
            self.core.selected_range.start
        };
        let cursor = if window.modifiers().shift {
            head
        } else {
            if !self.core.selected_range.is_empty() {
                self.core.selected_range.start
            } else {
                head
            }
        };

        let (line, col, _) = Self::line_col_for_index(&content, cursor);
        let preferred = self.core.preferred_column.get_or_insert(col);
        let target_line = line.saturating_sub(1);
        let new_index = Self::index_for_line_col(&content, target_line, *preferred);
        
        if window.modifiers().shift {
            self.select_to(new_index, cx);
        } else {
            self.set_cursor(new_index, cx);
        }
    }

    fn move_down(&mut self, _: &Down, window: &mut Window, cx: &mut Context<Self>) {
        if self.core.completion_active {
            if self.core.completion_index < self.core.completion_items.len().saturating_sub(1) {
                self.core.completion_index += 1;
                cx.notify();
            }
            return;
        }

        let content = self.core.content.to_string();
        let head = if self.core.selection_anchor == self.core.selected_range.start {
            self.core.selected_range.end
        } else {
            self.core.selected_range.start
        };
        let cursor = if window.modifiers().shift {
            head
        } else {
            if !self.core.selected_range.is_empty() {
                self.core.selected_range.end
            } else {
                head
            }
        };

        let (line, col, _) = Self::line_col_for_index(&content, cursor);
        let preferred = self.core.preferred_column.get_or_insert(col);
        let starts = Self::line_start_indices(&content);
        let max_line = starts.len().saturating_sub(1);
        let target_line = (line + 1).min(max_line);
        let new_index = Self::index_for_line_col(&content, target_line, *preferred);
        
        if window.modifiers().shift {
            self.select_to(new_index, cx);
        } else {
            self.set_cursor(new_index, cx);
        }
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
        } else {
            let delta = event.delta.pixel_delta(px(20.0));
            
            let bounds = self.layout.last_bounds.unwrap_or_default();
            let view_size = bounds.size;
            
            let lines: Vec<&str> = self.core.content.split('\n').collect();
            let line_count = lines.len().max(1);
            let total_height = self.layout.line_height() * line_count as f32;
            
            // Vertical scroll limit
            let max_scroll_y = (total_height - view_size.height + self.layout.line_height()).max(px(0.0));
            
            // Horizontal scroll limit
            let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
            let max_digits = line_count.to_string().len();
            let gutter_width = self.layout.gutter_width(max_digits);
            // Approximation: 0.75 * font_size per byte
            let char_width = self.layout.font_size * 0.75; 
            let content_width = gutter_width + px(40.0) + (max_line_len as f32 * char_width);
            let max_scroll_x = (content_width - view_size.width).max(px(0.0));

            self.layout.scroll(delta, point(max_scroll_x, max_scroll_y));
        }
        cx.notify();
    }

    // Helper functions
    fn line_start_indices(content: &str) -> Vec<usize> {
        let mut indices = vec![0];
        for (i, c) in content.char_indices() {
            if c == '\n' {
                indices.push(i + 1);
            }
        }
        indices
    }

    fn line_col_for_index(content: &str, index: usize) -> (usize, usize, usize) {
        let starts = Self::line_start_indices(content);
        let line_index = match starts.binary_search(&index) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let line_start = starts[line_index];
        let col = index - line_start;
        (line_index, col, line_start)
    }

    fn index_for_line_col(content: &str, line: usize, col: usize) -> usize {
        let starts = Self::line_start_indices(content);
        if line >= starts.len() {
            return content.len();
        }
        let line_start = starts[line];
        let line_end = if line + 1 < starts.len() {
            starts[line + 1] - 1
        } else {
            content.len()
        };
        let line_len = line_end - line_start;
        line_start + col.min(line_len)
    }

    fn prev_char_index(content: &str, index: usize) -> usize {
        if index == 0 {
            return 0;
        }
        let mut i = index - 1;
        while !content.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn next_char_index(content: &str, index: usize) -> usize {
        if index >= content.len() {
            return content.len();
        }
        let mut i = index + 1;
        while i < content.len() && !content.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        for (i, c) in self.core.content.char_indices() {
            if i >= offset {
                break;
            }
            utf16_offset += c.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        let start = self.offset_to_utf16(range.start);
        let end = self.offset_to_utf16(range.end);
        start..end
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        let mut byte_start = 0;
        let mut byte_end = 0;
        let mut current_utf16 = 0;
        
        for (i, c) in self.core.content.char_indices() {
            if current_utf16 == range_utf16.start {
                byte_start = i;
            }
            if current_utf16 == range_utf16.end {
                byte_end = i;
                break;
            }
            current_utf16 += c.len_utf16();
        }
        if current_utf16 < range_utf16.end {
            byte_end = self.core.content.len();
        }
        
        byte_start..byte_end
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
        
        // Simple tokenizer for demonstration
        for (i, c) in text.char_indices() {
            if !c.is_alphanumeric() && c != '_' && c != '#' {
                if start < i {
                    let word = &text[start..i];
                    let color = match word {
                        "int" | "char" | "float" | "double" | "bool" | "void" | "long" | "short" | "signed" | "unsigned" => Some(rgb(0x569cd6)), // Type: Blue
                        "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" | "break" | "continue" | "return" | "goto" => Some(rgb(0xc586c0)), // Control: Purple
                        "struct" | "class" | "enum" | "union" | "typedef" | "typename" | "template" | "namespace" | "using" => Some(rgb(0x569cd6)), // Keyword: Blue
                        "public" | "private" | "protected" | "virtual" | "override" | "static" | "const" | "inline" | "friend" => Some(rgb(0x569cd6)), // Modifier: Blue
                        "true" | "false" | "nullptr" | "this" | "new" | "delete" | "sizeof" | "operator" | "explicit" | "noexcept" => Some(rgb(0x569cd6)), // Keyword: Blue
                        "#include" | "#define" | "#ifdef" | "#ifndef" | "#endif" | "#pragma" => Some(rgb(0xc586c0)), // Preprocessor: Purple
                        _ => None,
                    };
                    if let Some(c) = color {
                        highlights.push((start..i, c.into()));
                    }
                }
                start = i + c.len_utf8();
            }
        }
        // Last word
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
                    color: rgb(0xcccccc).into(), // Default text color
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
        Some(self.core.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.core.selected_range),
            reversed: false,
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
            .unwrap_or(self.core.selected_range.clone());

        self.core.content =
            self.core.content[0..range.start].to_owned() + new_text + &self.core.content[range.end..];
        let new_cursor = range.start + new_text.len();
        self.core.selected_range = new_cursor..new_cursor;
        self.core.selection_anchor = new_cursor;
        self.core.marked_range = None;
        self.core.preferred_column = None;
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
            .unwrap_or(self.core.selected_range.clone());

        self.core.content =
            self.core.content[0..range.start].to_owned() + new_text + &self.core.content[range.end..];
        if !new_text.is_empty() {
            self.core.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.core.marked_range = None;
        }
        self.core.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| range.start + new_range.start..range.start + new_range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.core.selection_anchor = self.core.selected_range.start;
        self.core.preferred_column = None;
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
        let content = self.core.content.as_str();
        
        // Calculate max digits
        let line_count = content.split('\n').count().max(1);
        let max_digits = line_count.to_string().len();

        let (line_index, _, line_start) = Self::line_col_for_index(content, range.start);
        let line_height = self.layout.line_height();
        let text_x = self.layout.text_x(bounds, max_digits);
        let y = self.layout.line_y(bounds, line_index);
        let starts = Self::line_start_indices(content);
        let line_end = if line_index + 1 < starts.len() {
            starts[line_index + 1] - 1
        } else {
            content.len()
        };
        let line_text = &content[line_start..line_end];
        let line = Self::shape_code_line(window, line_text, self.layout.font_size);
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
        let content = self.core.content.as_str();
        
        // Calculate max digits
        let line_count = content.split('\n').count().max(1);
        let max_digits = line_count.to_string().len();

        let _line_height = self.layout.line_height();
        let text_x = self.layout.text_x(bounds, max_digits);
        
        let mut line_index = self.layout.line_index_for_y(bounds, point.y);
        let starts = Self::line_start_indices(content);
        if starts.is_empty() {
            line_index = 0;
        } else if line_index >= starts.len() {
            line_index = starts.len() - 1;
        }
        let line_start = starts.get(line_index).copied().unwrap_or(0);
        let line_end = if line_index + 1 < starts.len() {
            starts[line_index + 1] - 1
        } else {
            content.len()
        };
        let line_text = &content[line_start..line_end];
        let line = Self::shape_code_line(window, line_text, self.layout.font_size);
        let local_x = point.x - text_x;
        let utf8_index = line.index_for_x(local_x).unwrap_or(line_text.len());
        Some(self.offset_to_utf16(line_start + utf8_index))
    }
}

impl CodeEditor {
    fn index_for_point(&self, point: Point<Pixels>, window: &Window) -> Option<usize> {
        let bounds = self.layout.last_bounds?;
        let content = self.core.content.as_str();
        
        // Calculate max digits
        let line_count = content.split('\n').count().max(1);
        let max_digits = line_count.to_string().len();
        
        let text_x = self.layout.text_x(bounds, max_digits);
        
        let mut line_index = self.layout.line_index_for_y(bounds, point.y);
        let starts = Self::line_start_indices(content);
        if starts.is_empty() {
            line_index = 0;
        } else if line_index >= starts.len() {
            line_index = starts.len() - 1;
        }
        let line_start = starts.get(line_index).copied().unwrap_or(0);
        let line_end = if line_index + 1 < starts.len() {
            starts[line_index + 1] - 1
        } else {
            content.len()
        };
        let line_text = &content[line_start..line_end];
        let line = Self::shape_code_line(window, line_text, self.layout.font_size);
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
        if let Some(index) = self.index_for_point(event.position, window) {
            if event.modifiers.shift {
                self.select_to(index, cx);
            } else {
                self.set_cursor(index, cx);
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_action(cx.listener(Self::delete_line))
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
            
            let (layout, content, selection_anchor, selected_range, completion_active, completion_items, completion_index) = {
                let state = editor.read(cx);
                (
                    state.layout, 
                    state.core.content.clone(), 
                    state.core.selection_anchor, 
                    state.core.selected_range.clone(),
                    state.core.completion_active,
                    state.core.completion_items.clone(),
                    state.core.completion_index
                )
            };

            let font_size = layout.font_size;
            let line_height = layout.line_height();

            let lines: Vec<&str> = content.split('\n').collect();
            let line_count = lines.len().max(1);
            let max_digits = line_count.to_string().len();

            let text_x = layout.text_x(bounds, max_digits);
            // number_x will be calculated per line for right alignment

            let head = if selection_anchor == selected_range.start {
                selected_range.end
            } else {
                selected_range.start
            };

            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                let starts = CodeEditor::line_start_indices(&content);
                let selection = selected_range.clone();
                let (current_line, _, _) = CodeEditor::line_col_for_index(&content, head);

                let gutter_width = layout.gutter_width(max_digits);
                let text_area_bounds = Bounds::from_corners(
                    point(bounds.left() + gutter_width, bounds.top()),
                    bounds.bottom_right()
                );

                let start_line = layout.line_index_for_y(bounds, bounds.top());
                let end_line = (layout.line_index_for_y(bounds, bounds.bottom()) + 1).min(line_count);

                // 1. Draw Global Backgrounds (Current Line Highlight)
                // This covers full width including gutter
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

                // 2. Draw Gutter (Line Numbers)
                // Clipped by outer bounds (editor bounds)
                for i in start_line..end_line {
                    let y = layout.line_y(bounds, i);
                    let number_text = format!("{}", i + 1);
                    let number_line =
                        CodeEditor::shape_line(window, &number_text, rgb(0xff8b949e).into(), font_size);
                    
                    // Right align numbers in gutter
                    // Gutter ends at bounds.left() + gutter_width
                    // Padding is 8px (right padding)
                    let number_width = number_line.width;
                    let number_x = bounds.left() + gutter_width - px(8.0) - number_width;

                    number_line
                        .paint(point(number_x, y), line_height, window, cx)
                        .ok();
                }

                // 3. Draw Text Area (Content + Selection + Cursor)
                // Clipped by text_area_bounds (excludes gutter)
                window.with_content_mask(Some(ContentMask { bounds: text_area_bounds }), |window| {
                    for i in start_line..end_line {
                        let line_text = lines.get(i).copied().unwrap_or("");
                        let y = layout.line_y(bounds, i);

                        // Draw Selection Background
                        if !selection.is_empty() {
                            let line_start = starts.get(i).copied().unwrap_or(0);
                            let line_end_incl_newline = if i + 1 < starts.len() {
                                starts[i + 1]
                            } else {
                                content.len()
                            };

                            let sel_start = selection.start.max(line_start);
                            let sel_end = selection.end.min(line_end_incl_newline);

                            if sel_start < sel_end {
                                let start_in_line = sel_start - line_start;
                                let end_in_line = sel_end - line_start;
                                let line_len = line_text.len();

                                let shape_start = start_in_line.min(line_len);
                                let shape_end = end_in_line.min(line_len);

                                let text_line_shape = CodeEditor::shape_code_line(window, line_text, font_size);
                                let start_x = text_line_shape.x_for_index(shape_start);
                                let mut end_x = text_line_shape.x_for_index(shape_end);

                                if end_in_line > line_len {
                                    end_x += px(10.0); // Visual width for newline
                                }

                                let rect_bounds = Bounds::from_corners(
                                    point(text_x + start_x, y),
                                    point(text_x + end_x, y + line_height)
                                );
                                window.paint_quad(fill(rect_bounds, rgba(0x264f78aa)));
                            }
                        }

                        // Draw Text
                        let text_line = CodeEditor::shape_code_line(window, line_text, font_size);
                        text_line
                            .paint(point(text_x, y), line_height, window, cx)
                            .ok();
                    }

                    // Draw Cursor (only if visible)
                    let (line, _col, line_start) = CodeEditor::line_col_for_index(&content, head);
                    if line >= start_line && line < end_line {
                        let line_text = lines.get(line).copied().unwrap_or("");
                        let line_shape = CodeEditor::shape_code_line(window, line_text, font_size);
                        let local_index = head.saturating_sub(line_start);
                        let cursor_x = text_x + line_shape.x_for_index(local_index);
                        let cursor_y = layout.line_y(bounds, line);
                        let cursor_bounds = Bounds::new(point(cursor_x, cursor_y), size(px(1.0), line_height));
                        window.paint_quad(fill(cursor_bounds, rgb(0xffffffff)));
                    }
                    // Draw Completion Menu
                    if completion_active && !completion_items.is_empty() {
                        let (line, _, line_start) = CodeEditor::line_col_for_index(&content, head);
                        let line_text = lines.get(line).copied().unwrap_or("");
                        let line_shape = CodeEditor::shape_code_line(window, line_text, font_size);
                        let local_index = head.saturating_sub(line_start);
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
                        
                        // Draw menu background
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
                            
                            // Highlight selected item
                            if i == completion_index {
                                window.paint_quad(fill(item_bounds, rgb(0x04395e)));
                            }
                            
                            // Draw Icon
                            let icon_bounds = Bounds::new(
                                point(menu_x + px(4.0), item_y),
                                size(px(20.0), item_height)
                            );
                            let icon_text = item.kind.icon_text();
                            let icon_color = item.kind.color();
                            let icon_line = CodeEditor::shape_line(window, icon_text, icon_color, font_size);
                            // Center icon vertically
                            let icon_height = icon_line.ascent + icon_line.descent;
                            let icon_y = item_y + (item_height - icon_height) / 2.0;
                            icon_line.paint(point(menu_x + px(6.0), icon_y), item_height, window, cx).ok();

                            // Draw Label
                            let label_line = CodeEditor::shape_line(window, &item.label, rgb(0xcccccc).into(), font_size);
                            let label_height = label_line.ascent + label_line.descent;
                            let label_y = item_y + (item_height - label_height) / 2.0;
                            label_line.paint(point(menu_x + px(30.0), label_y), item_height, window, cx).ok();
                            
                            // Draw Detail
                            if !item.detail.is_empty() {
                                let detail_line = CodeEditor::shape_line(window, &item.detail, rgb(0x808080).into(), font_size);
                                let detail_height = detail_line.ascent + detail_line.descent;
                                let detail_y = item_y + (item_height - detail_height) / 2.0;
                                let detail_x = menu_x + menu_width - detail_line.width - px(8.0);
                                detail_line.paint(point(detail_x, detail_y), item_height, window, cx).ok();
                            }
                        }
                    }
                });
            });
        }
    )
    .size_full()
}
