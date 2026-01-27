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
        CtrlShiftTab
    ]
);

pub struct CodeEditor {
    focus_handle: FocusHandle,
    content: SharedString,
    selected_range: Range<usize>,
    marked_range: Option<Range<usize>>,
    last_bounds: Option<Bounds<Pixels>>,
    preferred_column: Option<usize>,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            selected_range: 0..0,
            marked_range: None,
            last_bounds: None,
            preferred_column: None,
        }
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
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

        for ch in self.content.chars() {
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

    fn shape_line(window: &Window, text: &str, color: Hsla) -> gpui::ShapedLine {
        let style = window.text_style();
        let run = TextRun {
            len: text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        window.text_system().shape_line(
            SharedString::from(text.to_string()),
            font_size,
            &[run],
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

    fn set_cursor(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_range = index..index;
        self.marked_range = None;
        self.preferred_column = None;
        cx.notify();
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let range = self.selected_range.clone();
        let content = self.content.to_string();
        let mut new_content = String::new();
        new_content.push_str(&content[0..range.start]);
        new_content.push_str(text);
        new_content.push_str(&content[range.end..]);
        let cursor = range.start + text.len();
        self.content = new_content.into();
        self.selected_range = cursor..cursor;
        self.marked_range = None;
        self.preferred_column = None;
        println!("{}", self.content);
        cx.notify();
    }

    fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        let mut new_content = String::new();
        new_content.push_str(&content[0..range.start]);
        new_content.push_str(&content[range.end..]);
        self.content = new_content.into();
        self.selected_range = range.start..range.start;
        self.marked_range = None;
        self.preferred_column = None;
        println!("{}", self.content);
        cx.notify();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = self.last_bounds else {
            return;
        };
        let content = self.content.to_string();
        let line_height = window.line_height();
        let gutter_width = px(52.0);
        let text_x = bounds.left() + gutter_width + px(8.0);
        let local_y = event.position.y - bounds.top();
        let mut line = (local_y / line_height).floor() as usize;
        let starts = Self::line_start_indices(&content);
        if starts.is_empty() {
            line = 0;
        } else if line >= starts.len() {
            line = starts.len() - 1;
        }
        let line_start = starts.get(line).copied().unwrap_or(0);
        let line_end = if line + 1 < starts.len() {
            starts[line + 1] - 1
        } else {
            content.len()
        };
        let line_text = &content[line_start..line_end];
        let line_shape = Self::shape_line(window, line_text, window.text_style().color);
        let local_x = event.position.x - text_x;
        let utf8_index = line_shape.index_for_x(local_x).unwrap_or(line_text.len());
        let cursor = line_start + utf8_index;
        self.set_cursor(cursor, cx);
    }

    fn backspace(&mut self, _: &Backspace, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        if !self.selected_range.is_empty() {
            self.delete_range(self.selected_range.clone(), cx);
            return;
        }
        let cursor = self.selected_range.end;
        if cursor == 0 {
            return;
        }
        let prev = Self::prev_char_index(&content, cursor);
        self.delete_range(prev..cursor, cx);
    }

    fn delete(&mut self, _: &Delete, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        if !self.selected_range.is_empty() {
            self.delete_range(self.selected_range.clone(), cx);
            return;
        }
        let cursor = self.selected_range.end;
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

    fn move_left(&mut self, _: &Left, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        let cursor = if self.selected_range.is_empty() {
            self.selected_range.end
        } else {
            self.selected_range.start
        };
        let prev = Self::prev_char_index(&content, cursor);
        self.set_cursor(prev, cx);
    }

    fn move_right(&mut self, _: &Right, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        let cursor = if self.selected_range.is_empty() {
            self.selected_range.end
        } else {
            self.selected_range.end
        };
        let next = Self::next_char_index(&content, cursor);
        self.set_cursor(next, cx);
    }

    fn move_up(&mut self, _: &Up, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        let cursor = self.selected_range.end;
        let (line, col, _) = Self::line_col_for_index(&content, cursor);
        let preferred = self.preferred_column.get_or_insert(col);
        let target_line = line.saturating_sub(1);
        let new_index = Self::index_for_line_col(&content, target_line, *preferred);
        self.selected_range = new_index..new_index;
        cx.notify();
    }

    fn move_down(&mut self, _: &Down, _window: &mut Window, cx: &mut Context<Self>) {
        let content = self.content.to_string();
        let cursor = self.selected_range.end;
        let (line, col, _) = Self::line_col_for_index(&content, cursor);
        let preferred = self.preferred_column.get_or_insert(col);
        let starts = Self::line_start_indices(&content);
        let max_line = starts.len().saturating_sub(1);
        let target_line = (line + 1).min(max_line);
        let new_index = Self::index_for_line_col(&content, target_line, *preferred);
        self.selected_range = new_index..new_index;
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
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
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
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range = None;
        self.preferred_column = None;
        println!("{}", self.content);
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
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| range.start + new_range.start..range.start + new_range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.preferred_column = None;
        println!("{}", self.content);
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
        let content = self.content.as_ref();
        let (line_index, _, line_start) = Self::line_col_for_index(content, range.start);
        let line_height = window.line_height();
        let gutter_width = px(52.0);
        let text_x = bounds.left() + gutter_width + px(8.0);
        let y = bounds.top() + line_height * line_index as f32;
        let starts = Self::line_start_indices(content);
        let line_end = if line_index + 1 < starts.len() {
            starts[line_index + 1] - 1
        } else {
            content.len()
        };
        let line_text = &content[line_start..line_end];
        let line = Self::shape_line(window, line_text, window.text_style().color);
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
        let bounds = self.last_bounds?;
        let content = self.content.as_ref();
        let line_height = window.line_height();
        let gutter_width = px(52.0);
        let text_x = bounds.left() + gutter_width + px(8.0);
        let local_y = point.y - bounds.top();
        let mut line_index = (local_y / line_height).floor() as usize;
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
        let line = Self::shape_line(window, line_text, window.text_style().color);
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
                editor.last_bounds = Some(bounds);
            });
            let state = editor.read(cx);
            let content = state.content.to_string();
            let cursor = state.selected_range.end;
            let line_height = window.line_height();
            let gutter_width = px(52.0);
            let text_x = bounds.left() + gutter_width + px(8.0);
            let number_x = bounds.left() + px(8.0);
            let lines: Vec<&str> = content.split('\n').collect();
            let line_count = lines.len().max(1);

            for i in 0..line_count {
                let line_text = lines.get(i).copied().unwrap_or("");
                let y = bounds.top() + line_height * i as f32;
                let number_text = format!("{}", i + 1);
                let number_line =
                    CodeEditor::shape_line(window, &number_text, rgb(0xff8b949e).into());
                let text_line =
                    CodeEditor::shape_line(window, line_text, window.text_style().color);
                number_line
                    .paint(point(number_x, y), line_height, window, cx)
                    .ok();
                text_line
                    .paint(point(text_x, y), line_height, window, cx)
                    .ok();
            }

            let (line, _col, line_start) = CodeEditor::line_col_for_index(&content, cursor);
            let line_text = lines.get(line).copied().unwrap_or("");
            let line_shape = CodeEditor::shape_line(window, line_text, window.text_style().color);
            let local_index = cursor.saturating_sub(line_start);
            let cursor_x = text_x + line_shape.x_for_index(local_index);
            let cursor_y = bounds.top() + line_height * line as f32;
            let cursor_bounds = Bounds::new(point(cursor_x, cursor_y), size(px(1.0), line_height));
            window.paint_quad(fill(cursor_bounds, rgb(0xffffffff)));
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
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::enter))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::ctrl_shift_tab))
            .on_modifiers_changed(cx.listener(Self::on_modifiers_changed))
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
