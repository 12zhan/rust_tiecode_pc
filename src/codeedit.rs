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
}

impl EditorCore {
    fn new() -> Self {
        Self {
            content: String::new(),
            selected_range: 0..0,
            selection_anchor: 0,
            marked_range: None,
            preferred_column: None,
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

    fn gutter_width(&self) -> Pixels {
        px(52.0)
    }

    fn text_x(&self, bounds: Bounds<Pixels>) -> Pixels {
        bounds.left() + self.gutter_width() + px(8.0) + self.scroll_offset.x
    }

    fn number_x(&self, bounds: Bounds<Pixels>) -> Pixels {
        bounds.left() + px(8.0)
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

    fn is_line_visible(&self, bounds: Bounds<Pixels>, line_index: usize) -> bool {
        let y = self.line_y(bounds, line_index);
        let line_height = self.line_height();
        y + line_height >= bounds.top() && y <= bounds.bottom()
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
        let text_x = self.layout.text_x(bounds);
        
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
        cx.notify();
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.core.insert_text(text);
        println!("{}", self.core.content);
        cx.notify();
    }

    fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        self.core.delete_range(range);
        println!("{}", self.core.content);
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
        self.insert_text("\n", cx);
    }

    fn tab(&mut self, _: &Tab, _window: &mut Window, cx: &mut Context<Self>) {
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
        let (line_index, _, line_start) = Self::line_col_for_index(content, range.start);
        let line_height = self.layout.line_height();
        let text_x = self.layout.text_x(bounds);
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
        let text_x = self.layout.text_x(bounds);
        
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
            
            let (layout, content, selection_anchor, selected_range) = {
                let state = editor.read(cx);
                (
                    state.layout, 
                    state.core.content.clone(), 
                    state.core.selection_anchor, 
                    state.core.selected_range.clone()
                )
            };

            let font_size = layout.font_size;
            let line_height = layout.line_height();
            let text_x = layout.text_x(bounds);
            let number_x = layout.number_x(bounds);

            let head = if selection_anchor == selected_range.start {
                selected_range.end
            } else {
                selected_range.start
            };

            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                let lines: Vec<&str> = content.split('\n').collect();
                let line_count = lines.len().max(1);
                let starts = CodeEditor::line_start_indices(&content);
                let selection = selected_range.clone();
                let (current_line, _, _) = CodeEditor::line_col_for_index(&content, head);

                let gutter_width = layout.gutter_width();
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
