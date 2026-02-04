use gpui::*;
pub mod file_tree;
pub mod modal;
pub mod tie_svg;
use std::ops::Range;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Theme {
    pub surface: Hsla,
    pub panel: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted_text: Hsla,
    pub accent: Hsla,
    pub accent_border: Hsla,
    pub input_bg: Hsla,
    pub input_border: Hsla,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            surface: rgb(0xff1f2428).into(),
            panel: rgb(0xff2d353b).into(),
            border: rgb(0xff3c474d).into(),
            text: rgb(0xffe6e0d9).into(),
            muted_text: rgb(0xffa9b1b6).into(),
            accent: rgb(0xff7daea3).into(),
            accent_border: rgb(0xff89b482).into(),
            input_bg: rgb(0xff20262b).into(),
            input_border: rgb(0xff424f57).into(),
        }
    }
}

const SELECT_OPTIONS: [&str; 3] = ["选项 A", "选项 B", "选项 C"];

actions!(
    component_library,
    [
        InputBackspace,
        InputDelete,
        InputLeft,
        InputRight,
        InputSelectAll
    ]
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Button,
    Input,
    Select,
    Switch,
    Progress,
}

#[derive(Clone, Copy)]
struct ComponentLayout {
    button: Bounds<Pixels>,
    input: Bounds<Pixels>,
    select: Bounds<Pixels>,
    switch_track: Bounds<Pixels>,
    progress_track: Bounds<Pixels>,
}

impl ComponentLayout {
    fn new(bounds: Bounds<Pixels>) -> Self {
        let padding = px(20.0);
        let gap = px(14.0);
        let title_height = px(22.0);
        let mut cursor_y = bounds.top() + padding + title_height + gap;
        let start_x = bounds.left() + padding;

        let button = Bounds::new(point(start_x, cursor_y), size(px(120.0), px(32.0)));
        cursor_y += button.size.height + gap;
        let input = Bounds::new(point(start_x, cursor_y), size(px(240.0), px(32.0)));
        cursor_y += input.size.height + gap;
        let select = Bounds::new(point(start_x, cursor_y), size(px(240.0), px(32.0)));
        cursor_y += select.size.height + gap;
        let switch_track = Bounds::new(point(start_x, cursor_y), size(px(48.0), px(24.0)));
        cursor_y += switch_track.size.height + gap;
        let progress_track = Bounds::new(point(start_x, cursor_y), size(px(240.0), px(10.0)));

        Self {
            button,
            input,
            select,
            switch_track,
            progress_track,
        }
    }

    fn progress_value(&self, x: Pixels) -> f32 {
        let left = self.progress_track.left();
        let right = self.progress_track.right();
        if right <= left {
            return 0.0;
        }
        let clamped = x.max(left).min(right);
        ((clamped - left) / (right - left)).clamp(0.0, 1.0)
    }

    fn hit_test(&self, position: Point<Pixels>) -> Option<Control> {
        if self.button.contains(&position) {
            return Some(Control::Button);
        }
        if self.input.contains(&position) {
            return Some(Control::Input);
        }
        if self.select.contains(&position) {
            return Some(Control::Select);
        }
        if self.switch_track.contains(&position) {
            return Some(Control::Switch);
        }
        if self.progress_track.contains(&position) {
            return Some(Control::Progress);
        }
        None
    }

    fn select_dropdown_bounds(&self, option_count: usize, option_height: Pixels) -> Bounds<Pixels> {
        let height = option_height * option_count as f32;
        Bounds::new(
            point(self.select.left(), self.select.bottom() + px(6.0)),
            size(self.select.size.width, height),
        )
    }

    fn select_option_bounds(
        &self,
        index: usize,
        option_height: Pixels,
        origin: Point<Pixels>,
    ) -> Bounds<Pixels> {
        Bounds::new(
            point(origin.x, origin.y + option_height * index as f32),
            size(self.select.size.width, option_height),
        )
    }
}

pub struct ComponentLibrary {
    theme: Theme,
    layout: Option<ComponentLayout>,
    hovered: Option<Control>,
    button_pressed: bool,
    button_count: u32,
    input_focused: bool,
    input_value: String,
    input_cursor: usize,
    input_selection: Option<Range<usize>>,
    input_marked_range: Option<Range<usize>>,
    select_index: usize,
    select_open: bool,
    select_anim: f32,
    select_anim_target: f32,
    select_option_hovered: Option<usize>,
    switch_on: bool,
    progress: f32,
    dragging_progress: bool,
    button_anim: f32,
    button_anim_target: f32,
    animating: bool,
    last_animation: Option<Instant>,
    focus_handle: FocusHandle,
}

pub enum ComponentLibraryEvent {
    ButtonClicked,
}

impl EventEmitter<ComponentLibraryEvent> for ComponentLibrary {}

impl ComponentLibrary {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            theme: Theme::dark(),
            layout: None,
            hovered: None,
            button_pressed: false,
            button_count: 0,
            input_focused: false,
            input_value: "输入内容".to_string(),
            input_cursor: "输入内容".len(),
            input_selection: None,
            input_marked_range: None,
            select_index: 0,
            select_open: false,
            select_anim: 0.0,
            select_anim_target: 0.0,
            select_option_hovered: None,
            switch_on: true,
            progress: 0.7,
            dragging_progress: false,
            button_anim: 0.0,
            button_anim_target: 0.0,
            animating: false,
            last_animation: None,
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(layout) = self.layout {
            let option_height = px(28.0);
            let dropdown_bounds =
                layout.select_dropdown_bounds(SELECT_OPTIONS.len(), option_height);
            let dropdown_height =
                dropdown_bounds.size.height * self.select_anim.clamp(0.0, 1.0);
            let visible_dropdown_bounds = Bounds::new(
                dropdown_bounds.origin,
                size(dropdown_bounds.size.width, dropdown_height),
            );
            if self.select_open && visible_dropdown_bounds.contains(&event.position) {
                let index = ((event.position.y - dropdown_bounds.top()) / option_height)
                    .floor()
                    .max(0.0) as usize;
                if index < SELECT_OPTIONS.len() {
                    self.select_index = index;
                    self.select_open = false;
                    self.select_anim_target = 0.0;
                    self.select_option_hovered = None;
                    self.ensure_animation(cx);
                    cx.notify();
                    return;
                }
            }
            match layout.hit_test(event.position) {
                Some(Control::Button) => {
                    self.button_pressed = true;
                    self.button_anim_target = 1.0;
                    self.ensure_animation(cx);
                    cx.notify();
                    return;
                }
                Some(Control::Input) => {
                    self.input_focused = true;
                    self.focus_handle.focus(window);
                    let text = self.input_value.clone();
                    let text_start = point(
                        layout.input.left() + px(12.0),
                        layout.input.top() + px(8.0),
                    );
                    let local_x = (event.position.x - text_start.x).max(px(0.0));
                    let line = shape_line(window, &text, self.theme.text, px(13.0));
                    let utf8_index = clamp_to_char_boundary(
                        &text,
                        line.index_for_x(local_x).unwrap_or(text.len()),
                    );
                    self.input_cursor = utf8_index;
                    self.input_selection = None;
                    cx.notify();
                    return;
                }
                Some(Control::Select) => {
                    self.select_open = !self.select_open;
                    self.select_anim_target = if self.select_open { 1.0 } else { 0.0 };
                    self.ensure_animation(cx);
                    cx.notify();
                    return;
                }
                Some(Control::Switch) => {
                    self.switch_on = !self.switch_on;
                    cx.notify();
                    return;
                }
                Some(Control::Progress) => {
                    self.dragging_progress = true;
                    self.progress = layout.progress_value(event.position.x);
                    cx.notify();
                    return;
                }
                None => {}
            }
        }
        self.input_focused = false;
        self.input_selection = None;
        if self.select_open {
            self.select_open = false;
            self.select_anim_target = 0.0;
            self.select_option_hovered = None;
            self.ensure_animation(cx);
        }
        cx.notify();
    }

    fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.button_pressed {
            if let Some(layout) = self.layout {
                if layout.button.contains(&event.position) {
                    self.button_count += 1;
                    self.input_value = format!("输入内容 {}", self.button_count);
                    self.input_cursor = self.input_value.len();
                    cx.emit(ComponentLibraryEvent::ButtonClicked);
                }
            }
        }
        self.button_pressed = false;
        self.button_anim_target = 0.0;
        self.ensure_animation(cx);
        self.dragging_progress = false;
        cx.notify();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(layout) = self.layout {
            if self.dragging_progress && event.pressed_button == Some(MouseButton::Left) {
                self.progress = layout.progress_value(event.position.x);
                cx.notify();
                return;
            }
            let next_hover = layout.hit_test(event.position);
            if next_hover != self.hovered {
                self.hovered = next_hover;
                cx.notify();
            }
            if self.select_open {
                let option_height = px(28.0);
                let dropdown_bounds =
                    layout.select_dropdown_bounds(SELECT_OPTIONS.len(), option_height);
                let dropdown_height =
                    dropdown_bounds.size.height * self.select_anim.clamp(0.0, 1.0);
                let visible_bounds = Bounds::new(
                    dropdown_bounds.origin,
                    size(dropdown_bounds.size.width, dropdown_height),
                );
                if visible_bounds.contains(&event.position) {
                    let index = ((event.position.y - dropdown_bounds.top()) / option_height)
                        .floor()
                        .max(0.0) as usize;
                    let next_hover = if index < SELECT_OPTIONS.len() {
                        Some(index)
                    } else {
                        None
                    };
                    if next_hover != self.select_option_hovered {
                        self.select_option_hovered = next_hover;
                        cx.notify();
                    }
                } else if self.select_option_hovered.is_some() {
                    self.select_option_hovered = None;
                    cx.notify();
                }
            }
        }
    }

    fn input_backspace(&mut self, _: &InputBackspace, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.input_focused {
            return;
        }
        if self.delete_selection_if_any() {
            cx.notify();
            return;
        }
        if self.input_cursor == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.input_value, self.input_cursor);
        self.input_value.replace_range(prev..self.input_cursor, "");
        self.input_cursor = prev;
        cx.notify();
    }

    fn input_delete(&mut self, _: &InputDelete, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.input_focused {
            return;
        }
        if self.delete_selection_if_any() {
            cx.notify();
            return;
        }
        if self.input_cursor >= self.input_value.len() {
            return;
        }
        let next = next_char_boundary(&self.input_value, self.input_cursor);
        self.input_value.replace_range(self.input_cursor..next, "");
        cx.notify();
    }

    fn input_left(&mut self, _: &InputLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.input_focused {
            return;
        }
        self.input_cursor = prev_char_boundary(&self.input_value, self.input_cursor);
        self.input_selection = None;
        cx.notify();
    }

    fn input_right(&mut self, _: &InputRight, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.input_focused {
            return;
        }
        self.input_cursor = next_char_boundary(&self.input_value, self.input_cursor);
        self.input_selection = None;
        cx.notify();
    }

    fn input_select_all(
        &mut self,
        _: &InputSelectAll,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.input_focused {
            return;
        }
        let len = self.input_value.len();
        self.input_selection = Some(0..len);
        self.input_cursor = len;
        cx.notify();
    }

    fn delete_selection_if_any(&mut self) -> bool {
        if let Some(range) = self.normalized_selection() {
            if range.start < range.end {
                self.input_value.replace_range(range.clone(), "");
                self.input_cursor = range.start;
                self.input_selection = None;
                self.input_marked_range = None;
                return true;
            }
        }
        false
    }

    fn normalized_selection(&self) -> Option<Range<usize>> {
        let range = self.input_selection.clone()?;
        let start = range.start.min(range.end).min(self.input_value.len());
        let end = range.start.max(range.end).min(self.input_value.len());
        Some(start..end)
    }

    fn ensure_animation(&mut self, cx: &mut Context<Self>) {
        if self.animating {
            return;
        }
        self.animating = true;
        cx.spawn(|entity: WeakEntity<ComponentLibrary>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    let keep_running = entity
                        .update(&mut cx, |state, cx| state.tick_animation(cx))
                        .unwrap_or(false);
                    if !keep_running {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    fn tick_animation(&mut self, cx: &mut Context<Self>) -> bool {
        let now = Instant::now();
        let dt = self
            .last_animation
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or(Duration::from_millis(16));
        self.last_animation = Some(now);
        let dt_sec = dt.as_secs_f32().min(0.05);

        let mut active = false;
        let button_before = self.button_anim;
        self.button_anim =
            smooth_approach(self.button_anim, self.button_anim_target, 16.0, dt_sec);
        if (self.button_anim - button_before).abs() > 0.0005 {
            active = true;
        }
        let select_before = self.select_anim;
        self.select_anim =
            smooth_approach(self.select_anim, self.select_anim_target, 12.0, dt_sec);
        if (self.select_anim - select_before).abs() > 0.0005 {
            active = true;
        }

        if !active
            && (self.button_anim - self.button_anim_target).abs() <= 0.001
            && (self.select_anim - self.select_anim_target).abs() <= 0.001
        {
            self.animating = false;
            self.last_animation = None;
            return false;
        }
        cx.notify();
        true
    }
}

impl Render for ComponentLibrary {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let theme = self.theme;
        let focus_handle = self.focus_handle.clone();

        div()
            .w_full()
            .h(px(240.0))
            .bg(theme.surface)
            .key_context("ComponentLibrary")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::input_backspace))
            .on_action(cx.listener(Self::input_delete))
            .on_action(cx.listener(Self::input_left))
            .on_action(cx.listener(Self::input_right))
            .on_action(cx.listener(Self::input_select_all))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(component_canvas(entity, focus_handle))
    }
}

impl EntityInputHandler for ComponentLibrary {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = utf16_range_to_byte_range(&self.input_value, range_utf16);
        adjusted_range.replace(byte_range_to_utf16_range(&self.input_value, range.clone()));
        Some(self.input_value[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if let Some(range) = self.normalized_selection() {
            return Some(UTF16Selection {
                range: byte_range_to_utf16_range(&self.input_value, range),
                reversed: false,
            });
        }
        let cursor_utf16 = byte_index_to_utf16(&self.input_value, self.input_cursor);
        Some(UTF16Selection {
            range: cursor_utf16..cursor_utf16,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.input_marked_range
            .as_ref()
            .map(|range| byte_range_to_utf16_range(&self.input_value, range.clone()))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.input_marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.input_focused {
            return;
        }
        let range = range_utf16
            .map(|range| utf16_range_to_byte_range(&self.input_value, range))
            .or(self.normalized_selection())
            .unwrap_or(self.input_cursor..self.input_cursor);
        let start = range.start.min(self.input_value.len());
        let end = range.end.min(self.input_value.len());
        self.input_value.replace_range(start..end, new_text);
        self.input_cursor = start + new_text.len();
        self.input_selection = None;
        self.input_marked_range = None;
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
        if !self.input_focused {
            return;
        }
        let range = range_utf16
            .map(|range| utf16_range_to_byte_range(&self.input_value, range))
            .or(self.normalized_selection())
            .unwrap_or(self.input_cursor..self.input_cursor);
        let start = range.start.min(self.input_value.len());
        let end = range.end.min(self.input_value.len());
        self.input_value.replace_range(start..end, new_text);
        if !new_text.is_empty() {
            let marked_end = start + new_text.len();
            self.input_marked_range = Some(start..marked_end);
        } else {
            self.input_marked_range = None;
        }
        if let Some(new_range_utf16) = new_selected_range_utf16 {
            let new_range = utf16_range_to_byte_range(new_text, new_range_utf16);
            let sel_start = (start + new_range.start).min(self.input_value.len());
            let sel_end = (start + new_range.end).min(self.input_value.len());
            self.input_selection = Some(sel_start..sel_end);
            self.input_cursor = sel_end;
        } else {
            self.input_selection = None;
            self.input_cursor = (start + new_text.len()).min(self.input_value.len());
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
        let layout = self.layout?;
        let target_bounds = if layout.input.size.width > px(0.0) {
            layout.input
        } else {
            bounds
        };
        let range = utf16_range_to_byte_range(&self.input_value, range_utf16);
        let font_size = px(13.0);
        let line_height = font_size * 1.4;
        let text_x = target_bounds.left() + px(12.0);
        let text_y = target_bounds.top() + px(8.0);
        let line = shape_line(window, &self.input_value, self.theme.text, font_size);
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
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let layout = self.layout?;
        let font_size = px(13.0);
        let text_x = layout.input.left() + px(12.0);
        let line = shape_line(window, &self.input_value, self.theme.text, font_size);
        let local_x = (point.x - text_x).max(px(0.0));
        let utf8_index = clamp_to_char_boundary(
            &self.input_value,
            line.index_for_x(local_x).unwrap_or(self.input_value.len()),
        );
        Some(byte_index_to_utf16(&self.input_value, utf8_index))
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

fn paint_text(
    window: &mut Window,
    text: &str,
    position: Point<Pixels>,
    line_height: Pixels,
    color: Hsla,
    font_size: Pixels,
    cx: &mut App,
) {
    let line = shape_line(window, text, color, font_size);
    line.paint(position, line_height, window, cx).ok();
}

fn paint_text_centered(
    window: &mut Window,
    text: &str,
    bounds: Bounds<Pixels>,
    color: Hsla,
    font_size: Pixels,
    cx: &mut App,
) {
    let line_height = font_size * 1.4;
    let line = shape_line(window, text, color, font_size);
    let x = bounds.left() + (bounds.size.width - line.width) / 2.0;
    let y = bounds.top() + (bounds.size.height - line_height) / 2.0;
    line.paint(point(x, y), line_height, window, cx).ok();
}

fn component_canvas(
    entity: Entity<ComponentLibrary>,
    focus_handle: FocusHandle,
) -> impl IntoElement {
    canvas(
        |bounds, _window, _cx| bounds,
        move |bounds, _layout, window, cx| {
            let layout = ComponentLayout::new(bounds);
            entity.update(cx, |state, _cx| {
                state.layout = Some(layout);
            });
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(layout.input, entity.clone()),
                cx,
            );

            let (
                theme,
                hovered,
                input_focused,
                input_value,
                input_cursor,
                select_index,
                select_open,
                select_anim,
                select_option_hovered,
                switch_on,
                progress,
                button_anim,
            ) = {
                let state = entity.read(cx);
                (
                    state.theme,
                    state.hovered,
                    state.input_focused,
                    state.input_value.clone(),
                    state.input_cursor,
                    state.select_index,
                    state.select_open,
                    state.select_anim,
                    state.select_option_hovered,
                    state.switch_on,
                    state.progress,
                    state.button_anim,
                )
            };

            window.paint_quad(fill(bounds, theme.surface));

            let font_size = px(13.0);
            let line_height = font_size * 1.4;
            let padding = px(20.0);

            paint_text(
                window,
                "自绘组件库",
                point(bounds.left() + padding, bounds.top() + padding),
                line_height,
                theme.text,
                font_size + px(2.0),
                cx,
            );

            let button_base_color = if hovered == Some(Control::Button) {
                rgb(0xff88b9ae).into()
            } else {
                theme.accent
            };
            let mut button_quad = fill(layout.button, button_base_color);
            button_quad.border_widths = Edges::all(px(1.0));
            button_quad.border_color = theme.accent_border.into();
            button_quad.corner_radii = Corners::all(px(6.0));
            window.paint_quad(button_quad);
            if button_anim > 0.001 {
                let overlay_alpha = (button_anim * 0.18).min(0.18);
                let mut overlay = fill(layout.button, hsla(0.0, 0.0, 0.0, overlay_alpha));
                overlay.corner_radii = Corners::all(px(6.0));
                window.paint_quad(overlay);
            }
            paint_text_centered(
                window,
                "按钮",
                layout.button,
                rgb(0xff102022).into(),
                font_size,
                cx,
            );

            let input_border = if input_focused {
                theme.accent
            } else if hovered == Some(Control::Input) {
                theme.border
            } else {
                theme.input_border
            };
            let mut input_quad = fill(layout.input, theme.input_bg);
            input_quad.border_widths = Edges::all(px(1.0));
            input_quad.border_color = input_border.into();
            input_quad.corner_radii = Corners::all(px(6.0));
            window.paint_quad(input_quad);
            let input_text = if input_value.is_empty() {
                "输入框"
            } else {
                &input_value
            };
            let input_color = if input_focused {
                theme.text
            } else {
                theme.muted_text
            };
            paint_text(
                window,
                input_text,
                point(layout.input.left() + px(12.0), layout.input.top() + px(8.0)),
                line_height,
                input_color,
                font_size,
                cx,
            );
            if input_focused {
                let caret_text = &input_value;
                let caret_line = shape_line(window, caret_text, theme.text, font_size);
                let caret_x = layout.input.left()
                    + px(12.0)
                    + caret_line.x_for_index(clamp_to_char_boundary(
                        caret_text,
                        input_cursor.min(caret_text.len()),
                    ));
                let caret_top = layout.input.top() + px(7.0);
                let caret_bottom = caret_top + line_height;
                let caret_bounds = Bounds::from_corners(
                    point(caret_x, caret_top),
                    point(caret_x + px(1.5), caret_bottom),
                );
                window.paint_quad(fill(caret_bounds, theme.text));
            }

            let select_border = if hovered == Some(Control::Select) {
                theme.accent
            } else {
                theme.border
            };
            let mut select_quad = fill(layout.select, theme.panel);
            select_quad.border_widths = Edges::all(px(1.0));
            select_quad.border_color = select_border.into();
            select_quad.corner_radii = Corners::all(px(6.0));
            window.paint_quad(select_quad);
            let select_label = SELECT_OPTIONS[select_index];
            paint_text(
                window,
                select_label,
                point(layout.select.left() + px(12.0), layout.select.top() + px(8.0)),
                line_height,
                theme.text,
                font_size,
                cx,
            );
            paint_text(
                window,
                "▾",
                point(layout.select.right() - px(20.0), layout.select.top() + px(6.0)),
                line_height,
                theme.muted_text,
                font_size + px(2.0),
                cx,
            );

            let switch_track_color = if switch_on {
                theme.accent
            } else {
                theme.input_bg
            };
            let mut switch_quad = fill(layout.switch_track, switch_track_color);
            switch_quad.border_widths = Edges::all(px(1.0));
            switch_quad.border_color = theme.input_border.into();
            switch_quad.corner_radii = Corners::all(px(12.0));
            window.paint_quad(switch_quad);
            let thumb_offset = if switch_on { px(27.0) } else { px(3.0) };
            let thumb_bounds = Bounds::new(
                point(layout.switch_track.left() + thumb_offset, layout.switch_track.top() + px(3.0)),
                size(px(18.0), px(18.0)),
            );
            let mut thumb_quad = fill(thumb_bounds, rgb(0xffffffff));
            thumb_quad.corner_radii = Corners::all(px(9.0));
            window.paint_quad(thumb_quad);
            paint_text(
                window,
                "开关",
                point(layout.switch_track.right() + px(12.0), layout.switch_track.top() + px(4.0)),
                line_height,
                theme.text,
                font_size,
                cx,
            );

            let mut track_quad = fill(layout.progress_track, theme.input_bg);
            track_quad.corner_radii = Corners::all(px(5.0));
            window.paint_quad(track_quad);
            let progress_width = layout.progress_track.size.width * progress.clamp(0.0, 1.0);
            let progress_fill_bounds = Bounds::new(
                layout.progress_track.origin,
                size(progress_width, layout.progress_track.size.height),
            );
            let mut progress_quad = fill(progress_fill_bounds, theme.accent);
            progress_quad.corner_radii = Corners::all(px(5.0));
            window.paint_quad(progress_quad);
            let percent_text = format!("进度条 {}%", (progress * 100.0).round() as i32);
            paint_text(
                window,
                &percent_text,
                point(layout.progress_track.right() + px(12.0), layout.progress_track.top() - px(6.0)),
                line_height,
                theme.muted_text,
                font_size,
                cx,
            );

            if select_open || select_anim > 0.001 {
                let option_height = px(28.0);
                let dropdown_bounds =
                    layout.select_dropdown_bounds(SELECT_OPTIONS.len(), option_height);
                let dropdown_height =
                    dropdown_bounds.size.height * select_anim.clamp(0.0, 1.0);
                let animated_bounds = Bounds::new(
                    dropdown_bounds.origin,
                    size(dropdown_bounds.size.width, dropdown_height),
                );
                let mut dropdown_quad = fill(animated_bounds, theme.panel);
                dropdown_quad.border_widths = Edges::all(px(1.0));
                dropdown_quad.border_color = theme.border.into();
                dropdown_quad.corner_radii = Corners::all(px(6.0));
                window.paint_quad(dropdown_quad);

                window.with_content_mask(Some(ContentMask { bounds: animated_bounds }), |window| {
                    for (index, label) in SELECT_OPTIONS.iter().enumerate() {
                        let item_bounds = layout.select_option_bounds(
                            index,
                            option_height,
                            dropdown_bounds.origin,
                        );
                        if let Some(hovered_index) = select_option_hovered {
                            if hovered_index == index {
                                window.paint_quad(fill(item_bounds, rgba(0xffffff10)));
                            }
                        }
                        if index == select_index {
                            window.paint_quad(fill(item_bounds, rgba(0xffffff18)));
                        }
                        paint_text(
                            window,
                            label,
                            point(item_bounds.left() + px(12.0), item_bounds.top() + px(6.0)),
                            line_height,
                            theme.text,
                            font_size,
                            cx,
                        );
                    }
                });
            }
        },
    )
    .w_full()
    .h_full()
}

fn smooth_approach(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    let t = 1.0 - (-speed * dt).exp();
    current + (target - current) * t
}

fn clamp_to_char_boundary(text: &str, mut index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
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

fn next_char_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    let mut i = (index + 1).min(text.len());
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn utf16_range_to_byte_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = utf16_index_to_byte(text, range.start);
    let end = utf16_index_to_byte(text, range.end);
    start..end
}

fn byte_range_to_utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = byte_index_to_utf16(text, range.start);
    let end = byte_index_to_utf16(text, range.end);
    start..end
}

fn utf16_index_to_byte(text: &str, utf16_index: usize) -> usize {
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

fn byte_index_to_utf16(text: &str, byte_index: usize) -> usize {
    let mut count = 0;
    for (i, ch) in text.char_indices() {
        if i >= byte_index {
            break;
        }
        count += ch.len_utf16();
    }
    count
}
