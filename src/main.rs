#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod component;
mod editor;
pub mod lsp;

use component::{
    file_tree::{FileTree, file_icon, FileTreeEvent}, ComponentLibrary, InputBackspace, InputDelete, InputLeft, InputRight, InputSelectAll,
};
use editor::{
    Backspace, CodeEditor, Copy, CtrlShiftTab, Cut, Delete, DeleteLine, Down, Enter, Escape,
    FindNext, FindPrev, Left, Paste, Redo, Right, SelectAll, ShiftTab, Tab, ToggleFind, Undo, Up,
};
use anyhow::Result;
use gpui::*;
use log::*;
use std::fs;
use std::path::PathBuf;

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

#[allow(dead_code)]
static APP_ID: &str = "d8b8e2b1-0c9b-4b7e-8b8a-0c9b4b7e8b8a";
fn main() {
    env_logger::init();

    Application::new()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        })
        .run(|context: &mut App| {
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
            KeyBinding::new("escape", Escape, Some("CodeEditor")),
            KeyBinding::new("f3", FindNext, Some("CodeEditor")),
            KeyBinding::new("shift-f3", FindPrev, Some("CodeEditor")),
            KeyBinding::new("backspace", InputBackspace, Some("ComponentLibrary")),
            KeyBinding::new("delete", InputDelete, Some("ComponentLibrary")),
            KeyBinding::new("left", InputLeft, Some("ComponentLibrary")),
            KeyBinding::new("right", InputRight, Some("ComponentLibrary")),
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
            KeyBinding::new(
                &format!("{}-a", ctrl_cmd),
                InputSelectAll,
                Some("ComponentLibrary"),
            ),
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

                    let sample = "类 启动类\n{\n    方法 启动方法()\n    {\n        变量 list: 列表<文本> = 新建 列表<文本>()\n        list.\n    }\n}\n";
                    editor.set_content(sample.to_string(), cx);
                });
                let component_library = cx.new(ComponentLibrary::new);
                let file_tree = cx.new(|cx| FileTree::new(std::env::current_dir().unwrap_or(std::path::PathBuf::from(".")), cx));
                cx.new(|cx| {
                    let subscription = cx.subscribe(&file_tree, |this: &mut StartWindow, _emitter, event: &FileTreeEvent, cx| {
                        match event {
                            FileTreeEvent::OpenFile(path) => {
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    this.editor.update(cx, |editor, cx| {
                                        editor.set_content(content, cx);
                                    });
                                }
                            }
                        }
                    });

                    StartWindow {
                        editor,
                        component_library,
                        file_tree,
                        _subscriptions: vec![subscription],
                    }
                })
            },
        );
    });
}

struct StartWindow {
    editor: Entity<CodeEditor>,
    component_library: Entity<ComponentLibrary>,
    file_tree: Entity<FileTree>,
    _subscriptions: Vec<Subscription>,
}

impl Render for StartWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_tree_view = self.file_tree.read(cx);
        let file_tree = self.file_tree.clone();
        let is_dragging = file_tree_view.is_dragging();
        let drag_source = file_tree_view.drag_source();
        let mouse_position = file_tree_view.mouse_position();

        div()
            .bg(rgb(0xFFFFFFFF))
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .on_drop(move |paths: &ExternalPaths, _, cx| {
                if let Some(path) = paths.paths().first() {
                     if path.is_dir() {
                         println!("Dropping folder: {:?}", path);
                         file_tree.update(cx, |tree, cx| {
                             tree.set_root_path(path.clone(), cx);
                         });
                     }
                }
            })
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
            .child(self.component_library.clone())
            .child(
                div()
                    .flex_1()
                    .flex()
                    .w_full()
                    .bg(rgb(0xff2d353b))
                    .child(
                        div()
                            .w(px(250.0))
                            .h_full()
                            .border_r_1()
                            .border_color(rgb(0xff3c474d))
                            .bg(rgb(0xff252526))
                            .child(self.file_tree.clone())
                    )
                    .child(self.editor.clone()),
            )
            .child(div().w_full().h(px(30.0)).bg(rgb(0xff354246)))
            .child(
                if is_dragging {
                    if let Some(path) = drag_source {
                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        div()
                            .absolute()
                            .top(mouse_position.y)
                            .left(mouse_position.x + px(10.0))
                            .flex()
                            .items_center()
                            .bg(rgb(0x2d353b))
                            .border_1()
                            .border_color(rgb(0x454545))
                            .rounded_md()
                            .p(px(4.0))
                            .opacity(0.8)
                            .child(file_icon(&name))
                            .child(
                                div()
                                    .ml(px(4.0))
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xffffff))
                                    .child(name)
                            )
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }
                } else {
                    div().into_any_element()
                }
            )
    }
}
