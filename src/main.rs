#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod editor;
pub mod lsp;

use editor::{
    Backspace, CancelFind, CodeEditor, Copy, CtrlShiftTab, Cut, Delete, DeleteLine, Down, Enter,
    Decoration, DecorationColor, FindNext, FindPrev, Left, Paste, Redo, Right, SelectAll, ShiftTab,
    Tab, ToggleFind, Undo, Up,
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
            KeyBinding::new(&format!("{}-a", ctrl_cmd), SelectAll, Some("CodeEditor")),
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
                editor.update(cx, |editor, cx| {
                    if editor.core.content.len_bytes() != 0 {
                        return;
                    }

                    let sample = "@code\nint main() {\n    int x = 0;\n    return x;\n}\n@end\n\n类 启动类\n变量 name: 文本 = \"hello\"\n";
                    editor.set_content(sample.to_string(), cx);

                    fn find_nth(haystack: &str, needle: &str, n: usize) -> Option<usize> {
                        let mut idx = 0;
                        let mut found = 0usize;
                        while let Some(pos) = haystack[idx..].find(needle) {
                            let abs = idx + pos;
                            if found == n {
                                return Some(abs);
                            }
                            found += 1;
                            idx = abs + needle.len();
                            if idx >= haystack.len() {
                                break;
                            }
                        }
                        None
                    }

                    let mut decorations = Vec::new();
                    if let Some(start) = find_nth(sample, "int", 0) {
                        decorations.push(Decoration {
                            range: start..start + "int".len(),
                            color: DecorationColor::Red,
                            message: Some("错误示例：这里用红色波浪线".to_string()),
                        });
                    }
                    if let Some(start) = find_nth(sample, "return", 0) {
                        decorations.push(Decoration {
                            range: start..start + "return".len(),
                            color: DecorationColor::Yellow,
                            message: Some("警告示例：这里用黄色波浪线".to_string()),
                        });
                    }
                    if let Some(start) = find_nth(sample, "变量", 0) {
                        decorations.push(Decoration {
                            range: start..start + "变量".len(),
                            color: DecorationColor::Gray,
                            message: Some("提示示例：这里用灰色波浪线".to_string()),
                        });
                    }
                    if let Some(start) = find_nth(sample, "启动类", 0) {
                        decorations.push(Decoration {
                            range: start..start + "启动类".len(),
                            color: DecorationColor::Custom(0x64ff64cc),
                            message: Some("自定义颜色示例：这里用 Custom 颜色".to_string()),
                        });
                    }
                    if let Some(start) = find_nth(sample, "\"hello\"", 0) {
                        decorations.push(Decoration {
                            range: start..start + "\"hello\"".len(),
                            color: DecorationColor::Custom(0x66b2ffff),
                            message: Some("信息示例：悬浮会显示提示文本".to_string()),
                        });
                    }

                    editor.set_decorations(decorations, cx);
                });
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
