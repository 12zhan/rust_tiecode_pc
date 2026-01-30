#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod editor;
pub mod lsp;

use editor::{
    Backspace, CancelFind, CodeEditor, Copy, CtrlShiftTab, Cut, Delete, DeleteLine, Down, Enter,
    FindNext, FindPrev, Left, Paste, Redo, Right, ShiftTab, Tab, ToggleFind, Undo, Up,
};
use gpui::*;
use log::*;
#[allow(dead_code)]
static APP_ID: &str = "d8b8e2b1-0c9b-4b7e-8b8a-0c9b4b7e8b8a";
fn main() {
    env_logger::init();

    Application::new().run(|context: &mut App| {
        info!("tiecode for desktop start success!");

        // 获取平台来确定ctrl还是cmd
        let ctrl_cmd = cfg!(target_os = "macos").then(|| "cmd").unwrap_or("ctrl");

        let mut bindings = vec![
            KeyBinding::new("backspace", Backspace, Some("CodeEditor")),
            KeyBinding::new("delete", Delete, Some("CodeEditor")),
            KeyBinding::new("left", Left, Some("CodeEditor")),
            KeyBinding::new("right", Right, Some("CodeEditor")),
            KeyBinding::new("up", Up, Some("CodeEditor")),
            KeyBinding::new("down", Down, Some("CodeEditor")),
            KeyBinding::new("enter", Enter, Some("CodeEditor")),
            KeyBinding::new("tab", Tab, Some("CodeEditor")),
            KeyBinding::new("shift-tab", ShiftTab, Some("CodeEditor")),
            KeyBinding::new("escape", CancelFind, Some("CodeEditor")),
            KeyBinding::new("f3", FindNext, Some("CodeEditor")),
            KeyBinding::new("shift-f3", FindPrev, Some("CodeEditor")),
        ];

        // 3. 动态拼接并添加带修饰键的绑定
        bindings.extend([
            KeyBinding::new(
                &format!("{}-shift-k", ctrl_cmd),
                DeleteLine,
                Some("CodeEditor"),
            ),
            KeyBinding::new(
                &format!("{}-shift-tab", ctrl_cmd),
                CtrlShiftTab,
                Some("CodeEditor"),
            ),
            KeyBinding::new(&format!("{}-c", ctrl_cmd), Copy, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-x", ctrl_cmd), Cut, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-v", ctrl_cmd), Paste, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-z", ctrl_cmd), Undo, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-shift-z", ctrl_cmd), Redo, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-f", ctrl_cmd), ToggleFind, Some("CodeEditor")),
        ]);

        // 4. 注册所有绑定
        context.bind_keys(bindings);

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
                    .w_full()
                    .bg(rgb(0xff2d353b))
                    .child(self.editor.clone()),
            )
            .child(div().w_full().h(px(30.0)).bg(rgb(0xff354246)))
    }
}
