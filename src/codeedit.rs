use gpui::*;
use std::ops::Range;

actions!(
    code_editor,
    [
        Backspace,
        Delete,
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

struct EditorCore {
    content: String,
    selected_range: Range<usize>,
    selection_anchor: usize,
    marked_range: Option<Range<usize>>,
    preferred_column: Option<usize>,
    completion_active: bool,
    completion_items: Vec<CompletionItem>,
    completion_index: usize,
}

#[derive(Clone, Debug)]
struct CompletionItem {
    label: String,
    kind: CompletionKind,
    detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CompletionKind {
    Function,
    Variable,
    Class,
    Keyword,
}

impl CompletionKind {
    fn icon_text(&self) -> &'static str {
        match self {
            Self::Function => "F",
            Self::Variable => "V",
            Self::Class => "T",
            Self::Keyword => "K",
        }
    }

    fn color(&self) -> Hsla {
        match self {
            Self::Function => rgb(0xdcb628).into(), // Yellow
            Self::Variable => rgb(0xd02a8c).into(), // Magenta
            Self::Class => rgb(0xaaaaaa).into(),    // Gray
            Self::Keyword => rgb(0x569cd6).into(),  // Blue
        }
    }
}

const CPP_KEYWORDS: &[&str] = &[
    "int", "char", "float", "double", "bool", "void", "long", "short", "signed", "unsigned",
    "if", "else", "for", "while", "do", "switch", "case", "default", "break", "continue", "return", "goto",
    "struct", "class", "enum", "union", "typedef", "typename", "template", "namespace", "using",
    "public", "private", "protected", "virtual", "override", "static", "const", "inline", "friend",
    "true", "false", "nullptr", "this", "new", "delete", "sizeof", "operator", "explicit", "noexcept",
    "#include", "#define", "#ifdef", "#ifndef", "#endif", "#pragma"
];

impl EditorCore {
    fn new() -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_anchor: 0,
            marked_range: None,
            preferred_column: None,
            completion_active: false,
            completion_items: Vec::new(),
            completion_index: 0,
        }
    }

    fn set_cursor(&mut self, index: usize) {
        self.selected_range = index..index;
        self.selection_anchor = index;
        self.marked_range = None;
        self.preferred_column = None;
    }

    fn select_to(&mut self, index: usize) {
        let start = self.selection_anchor.min(index);
        let end = self.selection_anchor.max(index);
        self.selected_range = start..end;
        self.marked_range = None;
        self.preferred_column = None;
    }

    fn insert_text(&mut self, text: &str) {
        let range = self.selected_range.clone();
        let mut new_content = String::new();
        new_content.push_str(&self.content[0..range.start]);
        new_content.push_str(text);
        new_content.push_str(&self.content[range.end..]);
        let cursor = range.start + text.len();
        self.content = new_content;
        self.selected_range = cursor..cursor;
        self.selection_anchor = cursor;
        self.marked_range = None;
        self.preferred_column = None;
    }

    fn delete_range(&mut self, range: Range<usize>) {
        let mut new_content = String::new();
        new_content.push_str(&self.content[0..range.start]);
        new_content.push_str(&self.content[range.end..]);
        self.content = new_content;
        self.selected_range = range.start..range.start;
        self.selection_anchor = range.start;
        self.marked_range = None;
        self.preferred_column = None;
    }
}

#[derive(Clone, Copy)]
struct EditorLayout {
    font_size: Pixels,
    scroll_offset: Point<Pixels>,
    last_bounds: Option<Bounds<Pixels>>,
}

impl EditorLayout {
    fn new() -> Self {
        Self {
            font_size: px(14.0),
            scroll_offset: point(px(0.0), px(0.0)),
            last_bounds: None,
        }
    }

    fn line_height(&self) -> Pixels {
        self.font_size * 1.4
    }

    fn gutter_width(&self, max_digits: usize) -> Pixels {
        let digit_width = self.font_size * 0.75; // Approximation for digit width
        let padding = px(16.0); // 8px left + 8px right
        digit_width * (max_digits as f32) + padding
    }

    fn text_x(&self, bounds: Bounds<Pixels>, max_digits: usize) -> Pixels {
        bounds.left() + self.gutter_width(max_digits) + px(8.0) + self.scroll_offset.x
    }

    fn line_y(&self, bounds: Bounds<Pixels>, line_index: usize) -> Pixels {
        bounds.top() + self.line_height() * line_index as f32 + self.scroll_offset.y
    }

    fn line_index_for_y(&self, bounds: Bounds<Pixels>, y: Pixels) -> usize {
        let local_y = y - bounds.top() - self.scroll_offset.y;
        let line_height = self.line_height();
        if line_height <= px(0.0) {
            return 0;
        }
        (local_y / line_height).floor().max(0.0) as usize
    }

    fn scroll(&mut self, delta: Point<Pixels>) {
        self.scroll_offset = self.scroll_offset + delta;
        self.scroll_offset.y = self.scroll_offset.y.min(px(0.0));
        self.scroll_offset.x = self.scroll_offset.x.min(px(0.0));
    }

    fn zoom(&mut self, delta: Pixels) {
        self.font_size = (self.font_size + delta).max(px(8.0));
    }

}

pub struct CodeEditor {
    focus_handle: FocusHandle,
    core: EditorCore,
    layout: EditorLayout,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            core: EditorCore::new(),
            layout: EditorLayout::new(),
        }
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.core.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.core.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn shape_line(window: &Window, text: &str, color: Hsla, font_size: Pixels) -> gpui::ShapedLine {
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
        let mut cursor = 0;
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let len = chars.len();

        while cursor < len {
            let (start_byte, ch) = chars[cursor];

            // Comments //
            if ch == '/' && cursor + 1 < len && chars[cursor + 1].1 == '/' {
                let end_byte = text.len();
                highlights.push((start_byte..end_byte, rgb(0x6a9955).into())); // Green
                break;
            }

            // Strings "
            if ch == '"' {
                let mut end_idx = cursor + 1;
                while end_idx < len {
                    if chars[end_idx].1 == '"' && (end_idx == 0 || chars[end_idx - 1].1 != '\\') {
                        end_idx += 1;
                        break;
                    }
                    end_idx += 1;
                }
                let end_byte = if end_idx < len { chars[end_idx].0 } else { text.len() };
                highlights.push((start_byte..end_byte, rgb(0xce9178).into())); // Brown
                cursor = end_idx;
                continue;
            }

            // Keywords / Identifiers
            if ch.is_alphabetic() || ch == '_' || ch == '#' {
                let mut end_idx = cursor + 1;
                while end_idx < len {
                    let (_, c) = chars[end_idx];
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    end_idx += 1;
                }

                let end_byte = if end_idx < len { chars[end_idx].0 } else { text.len() };
                let word = &text[start_byte..end_byte];

                let color = match word {
                    "int" | "char" | "float" | "double" | "bool" | "void" | "long" | "short" | "signed" | "unsigned" |
                    "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" | "break" | "continue" | "return" | "goto" |
                    "struct" | "class" | "enum" | "union" | "typedef" | "typename" | "template" | "namespace" | "using" |
                    "public" | "private" | "protected" | "virtual" | "override" | "static" | "const" | "inline" | "friend" |
                    "true" | "false" | "nullptr" | "this" | "new" | "delete" | "sizeof" | "operator" | "explicit" | "noexcept" |
                    "#include" | "#define" | "#ifdef" | "#ifndef" | "#endif" | "#pragma" => Some(rgb(0x569cd6).into()), // Blue
                    _ => None,
                };

                if let Some(c) = color {
                    highlights.push((start_byte..end_byte, c));
                }

                cursor = end_idx;
                continue;
            }

            // Numbers
            if ch.is_ascii_digit() {
                let mut end_idx = cursor + 1;
                let mut has_dot = false;
                while end_idx < len {
                    let (_, c) = chars[end_idx];
                    if c == '.' && !has_dot {
                        has_dot = true;
                    } else if !c.is_ascii_digit() {
                        break;
                    }
                    end_idx += 1;
                }
                let end_byte = if end_idx < len { chars[end_idx].0 } else { text.len() };
                highlights.push((start_byte..end_byte, rgb(0xb5cea8).into())); // Light Green
                cursor = end_idx;
                continue;
            }

            cursor += 1;
        }

        highlights
    }

    fn shape_code_line(window: &Window, text: &str, font_size: Pixels) -> gpui::ShapedLine {
        let style = window.text_style();
        let font = style.font();
        let highlights = Self::highlight_cpp(text);
        
        let mut runs = Vec::new();
        let mut current_byte = 0;

        for (range, color) in highlights {
            if range.start > current_byte {
                runs.push(TextRun {
                    len: range.start - current_byte,
                    font: font.clone(),
                    color: rgb(0xffffffff).into(), // Default White
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
            runs.push(TextRun {
                len: range.len(),
                font: font.clone(),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
            current_byte = range.end;
        }

        if current_byte < text.len() {
            runs.push(TextRun {
                len: text.len() - current_byte,
                font: font.clone(),
                color: rgb(0xffffffff).into(), // Default White
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

    fn line_start_indices(text: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                starts.push(idx + 1);
            }
        }
        if starts.is_empty() {
            starts.push(0);
        }
        starts
    }

    fn line_col_for_index(text: &str, index: usize) -> (usize, usize, usize) {
        let index = index.min(text.len());
        let starts = Self::line_start_indices(text);
        let mut line = 0;
        for (i, start) in starts.iter().enumerate() {
            if *start > index {
                break;
            }
            line = i;
        }
        let line_start = starts[line];
        let col = text[line_start..index].chars().count();
        (line, col, line_start)
    }

    fn index_for_line_col(text: &str, line: usize, col: usize) -> usize {
        let starts = Self::line_start_indices(text);
        let line = line.min(starts.len().saturating_sub(1));
        let line_start = starts[line];
        let line_end = if line + 1 < starts.len() {
            starts[line + 1] - 1
        } else {
            text.len()
        };
        let line_text = &text[line_start..line_end];
        let mut idx = line_start;
        let mut count = 0;
        for (offset, ch) in line_text.char_indices() {
            if count >= col {
                break;
            }
            idx = line_start + offset + ch.len_utf8();
            count += 1;
        }
        idx
    }

    fn prev_char_index(text: &str, index: usize) -> usize {
        let mut prev = 0;
        for (i, _) in text.char_indices() {
            if i >= index {
                break;
            }
            prev = i;
        }
        prev
    }

    fn next_char_index(text: &str, index: usize) -> usize {
        for (i, _) in text.char_indices() {
            if i > index {
                return i;
            }
        }
        text.len()
    }

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

    fn select_to(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.select_to(index);
        cx.notify();
    }

    fn set_cursor(&mut self, index: usize, cx: &mut Context<Self>) {
        self.core.set_cursor(index);
        self.core.completion_active = false; // Hide completion on manual cursor move
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
                if !prefix.is_empty() {
                    let mut items = Vec::new();
                    
                    // Add mock data for demonstration if they match prefix
                    let mock_data = vec![
                        CompletionItem { label: "main".to_string(), kind: CompletionKind::Function, detail: ":void".to_string() },
                        CompletionItem { label: "ant".to_string(), kind: CompletionKind::Variable, detail: ":int".to_string() },
                        CompletionItem { label: "Demo".to_string(), kind: CompletionKind::Class, detail: "".to_string() },
                    ];

                    for item in mock_data {
                        if item.label.starts_with(prefix) && item.label != prefix {
                            items.push(item);
                        }
                    }

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
        if !self.core.completion_active || self.core.completion_items.is_empty() {
            return;
        }
        
        let item = self.core.completion_items[self.core.completion_index].clone();
        
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
        
        self.core.delete_range(word_start..cursor);
        self.core.insert_text(&item.label);
        
        self.core.completion_active = false;
        self.core.completion_items.clear();
        self.core.completion_index = 0;
        
        cx.notify();
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.core.insert_text(text);
        self.update_completion(cx);
        // println!("{}", self.core.content); // Removed spammy print
        cx.notify();
    }

    fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.core.delete_range(range);
        self.update_completion(cx);
        // println!("{}", self.core.content); // Removed spammy print
        cx.notify();
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
        self.insert_text("\t", cx);
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.modifiers().control {
            let delta = event.delta.pixel_delta(px(10.0)).y;
            self.layout.zoom(delta);
        } else {
            let delta = event.delta.pixel_delta(px(20.0));
            self.layout.scroll(delta);
        }
        cx.notify();
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
        self.core.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.core.marked_range = None;
        self.core.preferred_column = None;
        println!("{}", self.core.content);
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
        self.core.preferred_column = None;
        println!("{}", self.core.content);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
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

fn code_editor_canvas(editor: Entity<CodeEditor>, focus_handle: FocusHandle) -> impl IntoElement {
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
                        
                        // Dynamic sizing based on font size and content
                        let item_height = layout.line_height();
                        let font_size = layout.font_size;
                        
                        // Calculate width based on max item length (label + detail)
                        let max_len = completion_items.iter().map(|s| s.label.len() + s.detail.len()).max().unwrap_or(10);
                        let char_width = font_size * 0.75; // Approximation
                        let icon_size = item_height * 0.8;
                        let padding_x = px(8.0);
                        let menu_padding = px(4.0);
                        let content_width = (char_width * max_len as f32 + icon_size + padding_x * 3.0).max(px(150.0));
                        let menu_width = content_width + menu_padding * 2.0;

                        // Handle scrolling for completion items if needed, for now just show top 10
                        let start_index = if completion_index >= 10 {
                            completion_index - 9
                        } else {
                            0
                        };
                        let display_count = completion_items.len().min(10);
                        let menu_height = item_height * display_count as f32 + menu_padding * 2.0;
                        
                        let menu_bounds = Bounds::new(
                            point(menu_x, menu_y),
                            size(menu_width, menu_height)
                        );
                        
                        // Draw menu background with rounded corners, border
                        let mut menu_quad = fill(menu_bounds, rgb(0x252526));
                        menu_quad.border_widths = Edges::all(px(1.0));
                        menu_quad.border_color = rgb(0x454545).into();
                        menu_quad.corner_radii = Corners::all(px(6.0));
                        window.paint_quad(menu_quad);
                        
                        // Items
                        let items_to_show = completion_items.iter().skip(start_index).take(10);
                        for (i, item) in items_to_show.enumerate() {
                            let actual_index = start_index + i;
                            let item_y = menu_y + menu_padding + item_height * i as f32;
                            let item_bounds = Bounds::new(
                                point(menu_x + menu_padding, item_y),
                                size(content_width, item_height)
                            );
                            
                            if actual_index == completion_index {
                                let mut highlight_quad = fill(item_bounds, rgb(0x455056));
                                highlight_quad.corner_radii = Corners::all(px(4.0));
                                window.paint_quad(highlight_quad);
                            }
                            
                            // Draw Icon
                            let icon_padding = (item_height - icon_size) / 2.0;
                            let icon_rect = Bounds::new(
                                point(menu_x + menu_padding + px(4.0), item_y + icon_padding),
                                size(icon_size, icon_size)
                            );
                            window.paint_quad(fill(icon_rect, item.kind.color()));
                            
                            // Draw Icon Character
                            let icon_char_shape = CodeEditor::shape_line(
                                window,
                                item.kind.icon_text(),
                                rgb(0xffffff).into(), // White text for icon
                                font_size * 0.8 // Smaller font for icon
                            );
                            // Center char in icon rect
                            let char_x = icon_rect.left() + (icon_size - icon_char_shape.width) / 2.0;
                            // GPUI ShapedLine uses ascent/descent, not simple height.
                            // But since we are drawing text with `paint`, we need to find the right y to visually center it.
                            // Usually `paint` takes the baseline origin.
                            // To center visually: box_top + (box_height + (ascent - descent)) / 2  - ascent ??
                            // Or simpler: just align baseline to a calculated center line.
                            // Center line of icon box: item_y + item_height / 2.0
                            // Baseline offset: (ascent - descent) / 2.0 ?
                            // Let's try: item_y + (item_height + (ascent - descent))/2.0 - descent 
                            // Actually, let's just use item_y which is top of the row.
                            // The `paint` method on ShapedLine usually expects the origin to be the top-left or baseline-left depending on implementation.
                            // Looking at `paint` signature in other places: `paint(origin, line_height, ...)`
                            // If it takes line_height, it might handle vertical centering or just fill the line.
                            // Let's just use `item_y` as we did for other text, but maybe add a small offset if icon font is smaller.
                            // The icon font is 0.8 * font_size.
                            // Let's center it within the icon_rect.
                            // We can use `item_y + (item_height - icon_char_shape.ascent - icon_char_shape.descent)/2.0 + icon_char_shape.ascent`?
                            // Let's stick to simple `item_y` for now but shift x.
                            // And maybe shift y slightly if it looks off.
                            // Actually, let's just use the same `item_y` as the main text to align baselines if possible,
                            // but here we want to center inside the colored box.
                            
                            // Let's use `paint` at (char_x, item_y) and trust `line_height` handling or adjust.
                            // But wait, the previous code used `item_y + px(2.0)`.
                            // Let's try to center it better.
                            // `ShapedLine` has `ascent` and `descent`.
                            // Height ~ ascent + descent.
                            // We want to center (ascent + descent) within icon_size.
                            // Top of text relative to baseline is -ascent. Bottom is descent.
                            // Center of text relative to baseline is (descent - ascent) / 2.
                            // Center of icon_rect relative to item_top is icon_padding + icon_size/2.
                            // We want baseline y such that: baseline_y + (descent - ascent)/2 = item_y + icon_padding + icon_size/2
                            // => baseline_y = item_y + icon_padding + icon_size/2 - (descent - ascent)/2
                            
                            // However, `paint` takes `origin` which is usually the top-left of the line box (for standard line layout).
                            // If `paint` expects top-left, we can just calculate top-left to center the text height.
                            // Text height = ascent + descent.
                            // Top offset = (icon_size - (ascent + descent)) / 2.0.
                            // y = icon_rect.top() + Top offset.
                            
                            // Let's check what `paint` does. In `CodeEditor::render`, we use `paint(point(x, y), line_height, ...)`
                            // So it probably aligns based on line_height.
                            // Let's just use item_y for Y coordinate to keep it simple and aligned with the row for now.
                            // But since the font is smaller, we might need to push it down a bit to vertically center with the larger text?
                            // No, smaller font on same line_height usually aligns baseline or middle?
                            // Let's just try centering the `icon_char_shape` within `icon_rect` manually.
                            
                            let text_height = icon_char_shape.ascent + icon_char_shape.descent;
                            let y_offset = (icon_size - text_height) / 2.0;
                            let char_y = icon_rect.top() + y_offset; 
                            
                            // NOTE: shaped_line.paint() behavior depends on GPUI version. 
                            // If it draws from top-left of the line bounds:
                            icon_char_shape.paint(point(char_x, char_y), item_height, window, cx).ok();


                            // Draw Label
                            let text_shape = CodeEditor::shape_line(
                                window, 
                                &item.label, 
                                rgb(0xcccccc).into(), 
                                font_size
                            );
                            
                            let text_start_x = menu_x + menu_padding + icon_size + px(12.0);
                            text_shape.paint(point(text_start_x, item_y), item_height, window, cx).ok();
                            
                            // Draw Detail
                            if !item.detail.is_empty() {
                                let detail_shape = CodeEditor::shape_line(
                                    window, 
                                    &item.detail, 
                                    rgb(0x808080).into(), // Gray
                                    font_size
                                );
                                let label_width = text_shape.width;
                                detail_shape.paint(point(text_start_x + label_width, item_y), item_height, window, cx).ok();
                            }
                        }
                    }
                });
            });
        },
    )
    .w_full()
    .h_full()
}

impl Render for CodeEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = cx.entity();
        let focus_handle = self.focus_handle(cx);

        div()
            .key_context("CodeEditor")
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::ctrl_shift_tab))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_modifiers_changed(cx.listener(Self::on_modifiers_changed))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_action(cx.listener(Self::move_left))
            .on_action(cx.listener(Self::move_right))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .w_full()
            .h_full()
            .bg(rgb(0xff1f2428))
            .child(code_editor_canvas(editor, focus_handle))
    }
}

impl Focusable for CodeEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
