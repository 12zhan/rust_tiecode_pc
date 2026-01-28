#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod codeedit;
pub mod lsp;

use codeedit::{
    Backspace, CodeEditor, Copy, CtrlShiftTab, Cut, Delete, Down, Enter, Left, Paste, Right,
    ShiftTab, Tab, Up,
};
use gpui::*;
use log::*;
#[allow(dead_code)]
static APP_ID: &str = "d8b8e2b1-0c9b-4b7e-8b8a-0c9b4b7e8b8a";
fn main() {
    env_logger::init();

    Application::new().run(|context: &mut App| {
        info!("tiecode for desktop start success!");

        context.bind_keys([
            KeyBinding::new("backspace", Backspace, Some("CodeEditor")),
            KeyBinding::new("delete", Delete, Some("CodeEditor")),
            KeyBinding::new("left", Left, Some("CodeEditor")),
            KeyBinding::new("right", Right, Some("CodeEditor")),
            KeyBinding::new("up", Up, Some("CodeEditor")),
            KeyBinding::new("down", Down, Some("CodeEditor")),
            KeyBinding::new("enter", Enter, Some("CodeEditor")),
            KeyBinding::new("tab", Tab, Some("CodeEditor")),
            KeyBinding::new("shift-tab", ShiftTab, Some("CodeEditor")),
            KeyBinding::new("ctrl-shift-tab", CtrlShiftTab, Some("CodeEditor")),
            KeyBinding::new("ctrl-c", Copy, Some("CodeEditor")),
            KeyBinding::new("ctrl-x", Cut, Some("CodeEditor")),
            KeyBinding::new("ctrl-v", Paste, Some("CodeEditor")),
        ]);

        let bounds = Bounds::centered(None, size(px(1200.0), px(700.0)), context);
        let _ = context.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    appears_transparent: true,
                    ..TitlebarOptions::default()
                }),
                ..WindowOptions::default()
            },
            |_, cx| {
                let editor = cx.new(CodeEditor::new);
                cx.new(|_| StartWindow { editor })
            },
        );
    });
}

struct StartWindow {
    editor: Entity<CodeEditor>,
}

impl Render for StartWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .bg(rgb(0xFFFFFFFF))
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .child(
                div()
                    .w_full()
                    .h(px(30.0))
                    .bg(rgb(0xff232a2e))
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .justify_between()
                    .child(div())
                    .child(
                        div()
                            .w_16()
                            .h_full()
                            .bg(rgb(0xfffbfafd))
                            .window_control_area(WindowControlArea::Max),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .w_full()
                    .bg(rgb(0xff2d353b))
                    .child(self.editor.clone()),
            )
            .child(div().w_full().h(px(30.0)).bg(rgb(0xff354246)))
    }
}
