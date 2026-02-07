use gpui::*;
use tiecode_plugin_api::CommandContribution;

pub struct CommandPalette {
    pub focus_handle: FocusHandle,
    input: String,
    input_cursor: usize,
    input_selection: Option<std::ops::Range<usize>>,
    input_marked_range: Option<std::ops::Range<usize>>,
    selected_index: usize,
    all_commands: Vec<CommandContribution>,
    filtered_commands: Vec<CommandContribution>,
    list_state: ListState,
    visible: bool,
    input_bounds: Option<Bounds<Pixels>>,
}

pub enum CommandPaletteEvent {
    ExecuteCommand(String),
    Dismiss,
}

impl EventEmitter<CommandPaletteEvent> for CommandPalette {}

impl CommandPalette {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            input: String::new(),
            input_cursor: 0,
            input_selection: None,
            input_marked_range: None,
            selected_index: 0,
            all_commands: Vec::new(),
            filtered_commands: Vec::new(),
            list_state: ListState::new(0, ListAlignment::Top, px(24.0)), // Height of item
            visible: false,
            input_bounds: None,
        }
    }

    pub fn set_commands(&mut self, commands: Vec<CommandContribution>, cx: &mut Context<Self>) {
        self.all_commands = commands;
        self.update_filter(cx);
    }

    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.input.clear();
        self.input_cursor = 0;
        self.input_selection = None;
        self.input_marked_range = None;
        self.update_filter(cx);
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        self.input_selection = None;
        self.input_marked_range = None;
        cx.notify();
    }

    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(CommandPaletteEvent::Dismiss);
        self.hide(cx);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    fn byte_index_to_utf16(s: &str, byte_idx: usize) -> usize {
        let mut count = 0;
        for (i, ch) in s.char_indices() {
            if i >= byte_idx {
                break;
            }
            count += ch.len_utf16();
        }
        count
    }

    fn utf16_index_to_byte(s: &str, utf16_idx: usize) -> usize {
        let mut count = 0;
        for (byte_index, ch) in s.char_indices() {
            let next = count + ch.len_utf16();
            if next > utf16_idx {
                return byte_index;
            }
            count = next;
        }
        s.len()
    }

    fn utf16_range_to_byte_range(s: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let start = Self::utf16_index_to_byte(s, range.start);
        let end = Self::utf16_index_to_byte(s, range.end);
        start.min(s.len())..end.min(s.len())
    }

    fn byte_range_to_utf16_range(s: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let start = Self::byte_index_to_utf16(s, range.start.min(s.len()));
        let end = Self::byte_index_to_utf16(s, range.end.min(s.len()));
        start..end
    }

    fn update_filter(&mut self, cx: &mut Context<Self>) {
        if self.input.is_empty() {
            self.filtered_commands = self.all_commands.clone();
        } else {
            let input = self.input.clone();
            let mut scored: Vec<(i64, CommandContribution)> = self
                .all_commands
                .iter()
                .filter_map(|cmd| {
                    let score_title = Self::fuzzy_score(&input, &cmd.title);
                    let score_cmd = Self::fuzzy_score(&input, &cmd.command);
                    
                    match (score_title, score_cmd) {
                        (Some(s1), Some(s2)) => Some((s1.max(s2), cmd.clone())),
                        (Some(s), None) => Some((s, cmd.clone())),
                        (None, Some(s)) => Some((s, cmd.clone())),
                        (None, None) => None,
                    }
                })
                .collect();
            
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            
            self.filtered_commands = scored.into_iter().map(|(_, cmd)| cmd).collect();
        }
        self.selected_index = 0;
        self.list_state.reset(self.filtered_commands.len());
        cx.notify();
    }

    fn fuzzy_score(pattern: &str, text: &str) -> Option<i64> {
        if pattern.is_empty() {
            return Some(0);
        }
        
        let pattern_chars: Vec<char> = pattern.to_lowercase().chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        
        let mut pattern_idx = 0;
        let mut score: i64 = 0;
        let mut last_match_idx: Option<usize> = None;
        
        for (i, c) in text_chars.iter().enumerate() {
            if pattern_idx >= pattern_chars.len() {
                break;
            }
            
            let c_lower = c.to_lowercase().next().unwrap_or(*c);
            
            if c_lower == pattern_chars[pattern_idx] {
                pattern_idx += 1;
                score += 10;
                
                let is_start = i == 0 || text_chars[i-1].is_whitespace() || text_chars[i-1] == '_' || text_chars[i-1] == '-' || text_chars[i-1] == '.';
                if is_start {
                    score += 20;
                }
                
                if let Some(last) = last_match_idx {
                    if i == last + 1 {
                        score += 5;
                    }
                }
                
                last_match_idx = Some(i);
            }
        }
        
        if pattern_idx == pattern_chars.len() {
            score -= text.len() as i64;
            Some(score)
        } else {
            None
        }
    }

    fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.filtered_commands.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
        self.list_state.scroll_to_reveal_item(self.selected_index);
        cx.notify();
    }

    fn select_prev(&mut self, cx: &mut Context<Self>) {
        if self.filtered_commands.is_empty() {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.filtered_commands.len() - 1;
        }
        self.list_state.scroll_to_reveal_item(self.selected_index);
        cx.notify();
    }

    fn confirm_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(cmd) = self.filtered_commands.get(self.selected_index) {
            cx.emit(CommandPaletteEvent::ExecuteCommand(cmd.command.clone()));
            self.hide(cx);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();

        if key == "enter" {
            self.confirm_selection(cx);
            return;
        }

        if key == "escape" {
            self.dismiss(cx);
            return;
        }

        if key == "up" {
            self.select_prev(cx);
            return;
        }

        if key == "down" {
            self.select_next(cx);
            return;
        }

        if key == "backspace" {
            if let Some(marked) = self.input_marked_range.take() {
                let start = marked.start.min(self.input.len());
                let end = marked.end.min(self.input.len());
                if start < end {
                    self.input.replace_range(start..end, "");
                    self.input_cursor = start;
                }
            } else if let Some(sel) = self.input_selection.take() {
                let start = sel.start.min(self.input.len());
                let end = sel.end.min(self.input.len());
                if start < end {
                    self.input.replace_range(start..end, "");
                    self.input_cursor = start;
                }
            } else if self.input_cursor > 0 {
                let prev = prev_char_boundary(&self.input, self.input_cursor);
                self.input.replace_range(prev..self.input_cursor, "");
                self.input_cursor = prev;
            } else {
                self.input.pop();
                self.input_cursor = self.input.len();
            }
            self.input_marked_range = None;
            self.update_filter(cx);
            cx.notify();
            return;
        }

        if event.keystroke.modifiers.control
            || event.keystroke.modifiers.alt
            || event.keystroke.modifiers.platform
        {
            return;
        }
    }
}

fn prev_char_boundary(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    let mut i = index.saturating_sub(1);
    while i > 0 && !text.is_char_boundary(i) {
        i = i.saturating_sub(1);
    }
    i
}

fn clamp_to_char_boundary(text: &str, mut index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    while index > 0 && !text.is_char_boundary(index) {
        index = index.saturating_sub(1);
    }
    index
}

impl EntityInputHandler for CommandPalette {
    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.input_marked_range
            .as_ref()
            .map(|range| Self::byte_range_to_utf16_range(&self.input, range.clone()))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.input_marked_range = None;
    }

    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        adjusted_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = Self::utf16_range_to_byte_range(&self.input, range_utf16);
        adjusted_range.replace(Self::byte_range_to_utf16_range(&self.input, range.clone()));
        Some(self.input[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if let Some(sel) = self.input_selection.clone() {
            return Some(UTF16Selection {
                range: Self::byte_range_to_utf16_range(&self.input, sel),
                reversed: false,
            });
        }
        let cursor_utf16 = Self::byte_index_to_utf16(&self.input, self.input_cursor);
        Some(UTF16Selection {
            range: cursor_utf16..cursor_utf16,
            reversed: false,
        })
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| Self::utf16_range_to_byte_range(&self.input, r))
            .or(self.input_marked_range.clone())
            .or(self.input_selection.clone())
            .unwrap_or(self.input_cursor..self.input_cursor);
        let start = range.start.min(self.input.len());
        let end = range.end.min(self.input.len());
        self.input.replace_range(start..end, new_text);
        self.input_cursor = start + new_text.len();
        self.input_selection = None;
        self.input_marked_range = None;
        self.update_filter(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<std::ops::Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|r| Self::utf16_range_to_byte_range(&self.input, r))
            .or(self.input_marked_range.clone())
            .or(self.input_selection.clone())
            .unwrap_or(self.input_cursor..self.input_cursor);
        let start = range.start.min(self.input.len());
        let end = range.end.min(self.input.len());
        self.input.replace_range(start..end, new_text);
        if !new_text.is_empty() {
            let marked_end = start + new_text.len();
            self.input_marked_range = Some(start..marked_end);
        } else {
            self.input_marked_range = None;
        }
        if let Some(new_range_utf16) = new_selected_range_utf16 {
            let new_range = Self::utf16_range_to_byte_range(new_text, new_range_utf16);
            let sel_start = (start + new_range.start).min(self.input.len());
            let sel_end = (start + new_range.end).min(self.input.len());
            self.input_selection = Some(sel_start..sel_end);
            self.input_cursor = sel_end;
        } else {
            self.input_selection = None;
            self.input_cursor = (start + new_text.len()).min(self.input.len());
        }
        self.update_filter(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let target_bounds = self.input_bounds.unwrap_or(bounds);
        let range = Self::utf16_range_to_byte_range(&self.input, range_utf16);
        let font_size = px(13.0);
        let line_height = font_size * 1.4;
        let text_x = target_bounds.left() + px(12.0);
        let text_y = target_bounds.top() + px(4.0);
        let style = window.text_style();
        let run = TextRun {
            len: self.input.len(),
            font: style.font(),
            color: rgb(0xffcccccc).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window.text_system().shape_line(
            SharedString::from(self.input.clone()),
            font_size,
            &[run],
            None,
        );
        let start_x = line.x_for_index(range.start);
        let end_x = line.x_for_index(range.end);
        Some(Bounds::from_corners(
            point(text_x + start_x, text_y),
            point(text_x + end_x, text_y + line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.input_bounds?;
        let font_size = px(13.0);
        let text_x = bounds.left() + px(12.0);
        let local_x = (point.x - text_x).max(px(0.0));
        let style = _window.text_style();
        let run = TextRun {
            len: self.input.len(),
            font: style.font(),
            color: rgb(0xffcccccc).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = _window.text_system().shape_line(
            SharedString::from(self.input.clone()),
            font_size,
            &[run],
            None,
        );
        let utf8_index = clamp_to_char_boundary(
            &self.input,
            line.index_for_x(local_x).unwrap_or(self.input.len()),
        );
        Some(Self::byte_index_to_utf16(&self.input, utf8_index))
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        let theme_bg = rgb(0xff252526);
        let theme_border = rgb(0xff3c474d);
        let theme_text = rgb(0xffcccccc);
        let theme_selected = rgb(0xff37373d);

        let filtered_commands = self.filtered_commands.clone();
        let selected_index = self.selected_index;
        let palette = cx.entity();
        let input_focus = self.focus_handle.clone();

        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w_full()
            .h_full()
            .flex()
            .justify_center()
            .pt(px(40.0)) // Top offset
            .child(
                div() // Overlay backdrop
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .w_full()
                    .h_full()
                    .bg(rgba(0x00000080))
                    // Click outside to dismiss
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                         cx.stop_propagation();
                         this.dismiss(cx);
                    }))
            )
            .child(
                div() // Palette box
                    .w(px(600.0))
                    .max_h(px(400.0))
                    .bg(theme_bg)
                    .border_1()
                    .border_color(theme_border)
                    .rounded_lg()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .track_focus(&self.focus_handle)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        this.on_key_down(event, window, cx);
                    }))
                    .child(
                        // Input area
                        div()
                            .p(px(8.0))
                            .relative()
                            .child(
                                div()
                                    .w_full()
                                    .bg(rgb(0xff3c3c3c))
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(0xff007fd4))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_color(theme_text)
                                    .child(if self.input.is_empty() {
                                        "Type a command...".to_string()
                                    } else {
                                        self.input.clone()
                                    }),
                            )
                            .child(
                                canvas(
                                    |bounds, _window, _cx| bounds,
                                    {
                                        let palette = palette.clone();
                                        move |bounds, _layout, window, cx| {
                                            palette.update(cx, |this, _cx| {
                                                this.input_bounds = Some(bounds);
                                            });
                                            window.handle_input(
                                                &input_focus,
                                                ElementInputHandler::new(bounds, palette.clone()),
                                                cx,
                                            );
                                        }
                                    },
                                )
                                .absolute()
                                .top(px(8.0))
                                .left(px(8.0))
                                .right(px(8.0))
                                .h(px(24.0)),
                            )
                    )
                    .child(
                        // List area
                        list(self.list_state.clone(), move |index, _window, _cx| {
                            if index >= filtered_commands.len() {
                                return div().into_any_element();
                            }
                            let cmd = &filtered_commands[index];
                            let is_selected = index == selected_index;
                            
                            div()
                                .w_full()
                                .px(px(12.0))
                                .py(px(4.0))
                                .flex()
                                .justify_between()
                                .items_center()
                                .bg(if is_selected { theme_selected } else { theme_bg })
                                .text_color(theme_text)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .child(cmd.title.clone())
                                        .child(
                                            if let Some(cat) = &cmd.category {
                                                div()
                                                    .ml(px(8.0))
                                                    .text_size(px(10.0))
                                                    .text_color(rgb(0xff888888))
                                                    .child(cat.clone())
                                            } else {
                                                div()
                                            }
                                        )
                                )
                                .into_any_element()
                        })
                        .h_full()
                    )
            )
            .into_any_element()
    }
}
