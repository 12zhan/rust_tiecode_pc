#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod component;
mod editor;
mod plugin;
mod lsp;
mod panic_handler;

//DEMO

use component::{
    command_palette::{CommandPalette, CommandPaletteEvent},
    file_tree::{file_icon, FileTree, FileTreeEvent},
    modal::modal,
    popover::popover,
    tie_svg::tie_svg,
    status_bar::StatusBar,
};
use editor::{
    Backspace, CodeEditor, CodeEditorEvent, Copy, CtrlShiftTab, Cut, Delete, DeleteLine, Down, Enter, Escape,
    FindNext, FindPrev, GoToDefinition, FormatDocument, SignatureHelp, Left, Paste, Redo, Right, SelectAll, ShiftTab, Tab, ToggleFind, Undo, Up,
    IndentGuideHighlightColor,
};
use plugin::manager::PluginManager;
use tiecode_plugin_api::CommandContribution;
use anyhow::Result;
use gpui::*;
use log::*;
use image::GenericImageView;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

actions!(start_window, [ShowCommandPalette]);

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

fn default_assets_base() -> PathBuf {
    if let Ok(base) = std::env::var("TIECODE_ASSETS_BASE") {
        return PathBuf::from(base);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if cfg!(target_os = "macos") {
                let resources_dir = exe_dir.join("../Resources");
                if resources_dir.join("assets").is_dir() {
                    return resources_dir;
                }
            }

            if exe_dir.join("assets").is_dir() {
                return exe_dir.to_path_buf();
            }
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(windows)]
mod windows_console {
    #[link(name = "kernel32")]
    extern "system" {
        pub fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
    }
}

fn main() {
    #[cfg(windows)]
    unsafe {
        // Set console output code page to UTF-8 (65001) to fix garbled Chinese logs
        windows_console::SetConsoleOutputCP(65001);
    }

    panic_handler::init();
    env_logger::init();

    Application::new()
        .with_assets(Assets {
            base: default_assets_base(),
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
            KeyBinding::new("f12", GoToDefinition, Some("CodeEditor")),
            KeyBinding::new(&format!("{}-shift-space", ctrl_cmd), SignatureHelp, Some("CodeEditor")),
            KeyBinding::new("shift-alt-f", FormatDocument, Some("CodeEditor")),
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
            KeyBinding::new(&format!("{}-shift-p", ctrl_cmd), ShowCommandPalette, None),
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
                let editor = cx.new(|cx| CodeEditor::new(cx, None));
                editor.update(cx, |editor, _cx| {
                    // Indent guides: disable animation + bold, enable colorful palette.
                    editor.indent_guides.highlight.animate = false;
                    editor.indent_guides.thickness.highlighted = editor.indent_guides.thickness.normal;
                    editor.indent_guides.highlight.colors = IndentGuideHighlightColor::Palette(vec![
                        rgb(0x4ec9b0),
                        rgb(0x569cd6),
                        rgb(0xc586c0),
                        rgb(0xdcdcaa),
                        rgb(0xce9178),
                    ]);
                    editor.indent_guides.highlight.randomize_palette = true;
                });
                /* editor.update(cx, |editor, cx| {
                     if editor.core.content.len_bytes() != 0 {
                         return;
                     }

                    let sample = "类 启动类\n{\n    方法 启动方法()\n    {\n        变量 list: 列表<文本> = 新建 列表<文本>()\n        list.\n    }\n}\n";
                    editor.set_content(sample.to_string(), cx);
                }); */
                let file_tree = cx.new(|cx| FileTree::new(None, cx));
                let command_palette = cx.new(CommandPalette::new);
                let image_viewer = cx.new(|cx| crate::component::image_viewer::ImageViewer::new(cx));
                let markdown_viewer = cx.new(|cx| crate::component::markdown_viewer::MarkdownViewer::new(cx));
                let tool_panel = {
                    let ft = file_tree.clone();
                    cx.new(|cx| crate::component::tool_panel::ToolPanel::new(ft, cx))
                };
                let git_panel = cx.new(|cx| crate::component::git_panel::GitPanel::new(cx));
                let plugin_manager = cx.new(|_| PluginManager::new());
                let status_bar = cx.new(|cx| StatusBar::new(editor.clone(), cx));
                
                plugin_manager.update(cx, |manager: &mut PluginManager, _cx| {
                    manager.discover_plugins();
                    manager.command_registry.register(CommandContribution {
                        command: "file_tree.toggle".to_string(),
                        title: "Toggle File Tree".to_string(),
                        category: Some("View".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.undo".to_string(),
                        title: "Undo".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.redo".to_string(),
                        title: "Redo".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.cut".to_string(),
                        title: "Cut".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.copy".to_string(),
                        title: "Copy".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.paste".to_string(),
                        title: "Paste".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.select_all".to_string(),
                        title: "Select All".to_string(),
                        category: Some("Edit".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.save".to_string(),
                        title: "Save".to_string(),
                        category: Some("File".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.close".to_string(),
                        title: "Close Editor".to_string(),
                        category: Some("File".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "core.exit".to_string(),
                        title: "Exit".to_string(),
                        category: Some("File".to_string()),
                    });
                    manager.command_registry.register(CommandContribution {
                        command: "view.set_background".to_string(),
                        title: "Set Background Image".to_string(),
                        category: Some("View".to_string()),
                    });
                    manager.register_tool_page("git", "Git", Some(PathBuf::from("assets/git.svg")));
                });

                {
                    let pages = plugin_manager.read(cx).list_tool_pages().to_vec();
                    tool_panel.update(cx, |panel, cx| {
                        panel.attach_git_panel(git_panel.clone());
                        for p in pages {
                            panel.add_tool_page(p.id, p.label, p.icon_path);
                        }
                        cx.notify();
                    });
                }

                cx.new(|cx| {
                    let subscription = cx.subscribe(&file_tree, |this: &mut StartWindow, _emitter, event: &FileTreeEvent, cx| {
                        match event {
                            FileTreeEvent::OpenFile(path) => {
                                this.open_file_path(path.clone(), cx);
                            }
                            FileTreeEvent::ContextMenu { position, path, is_dir } => {
                                this.context_menu_open = true;
                                this.context_menu_position = *position;
                                this.context_menu_path = Some(path.clone());
                                this.context_menu_is_dir = *is_dir;
                                cx.notify();
                            }
                            FileTreeEvent::RequestMove { src, dst } => {
                                this.request_confirm(
                                    ConfirmAction::Move {
                                        src: src.clone(),
                                        dst: dst.clone(),
                                    },
                                    cx,
                                );
                            }
                            FileTreeEvent::RequestDelete { path, is_dir } => {
                                this.request_confirm(
                                    ConfirmAction::Delete {
                                        path: path.clone(),
                                        is_dir: *is_dir,
                                    },
                                    cx,
                                );
                            }
                        }
                    });

                    let editor_subscription = cx.subscribe(&editor, |this: &mut StartWindow, _emitter, event: &CodeEditorEvent, cx| {
                        match event {
                            CodeEditorEvent::OpenFile(path) => {
                                this.open_file_path(path.clone(), cx);
                            }
                        }
                    });

                    let palette_subscription = cx.subscribe(&command_palette, |this: &mut StartWindow, _emitter, event: &CommandPaletteEvent, cx| {
                        match event {
                            CommandPaletteEvent::Dismiss => {
                                this.command_palette.update(cx, |palette, cx| {
                                    palette.hide(cx);
                                });
                                this.needs_focus_restore = true;
                                cx.notify();
                            }
                            CommandPaletteEvent::ExecuteCommand(command_id) => {
                                this.execute_command(&command_id, cx);
                            }
                        }
                    });

                    StartWindow {
                        editor,
                        file_tree,
                        command_palette,
                        plugin_manager,
                        status_bar,
                        image_viewer,
                        markdown_viewer,
                        tool_panel,
                        file_tree_visible: true,
                        open_tabs: Vec::new(),
                        active_tab: None,
                        external_drag_position: point(px(0.0), px(0.0)),
                        external_drag_primary: None,
                        external_drag_is_dir: false,
                        external_drag_count: 0,
                        confirm_open: false,
                        confirm_action: None,
                        context_menu_open: false,
                        context_menu_position: point(px(0.0), px(0.0)),
                        context_menu_path: None,
                        context_menu_is_dir: false,
                        _subscriptions: vec![
                            subscription,
                            editor_subscription,
                            palette_subscription,
                        ],
                        needs_focus_restore: false,
                        needs_initial_focus: true,
                        background_image: None,
                    }
                })
            },
        );
    });
}

struct StartWindow {
    editor: Entity<CodeEditor>,
    file_tree: Entity<FileTree>,
    command_palette: Entity<CommandPalette>,
    plugin_manager: Entity<PluginManager>,
    status_bar: Entity<StatusBar>,
    image_viewer: Entity<crate::component::image_viewer::ImageViewer>,
    markdown_viewer: Entity<crate::component::markdown_viewer::MarkdownViewer>,
    tool_panel: Entity<crate::component::tool_panel::ToolPanel>,
    file_tree_visible: bool,
    open_tabs: Vec<PathBuf>,
    active_tab: Option<PathBuf>,
    external_drag_position: Point<Pixels>,
    external_drag_primary: Option<PathBuf>,
    external_drag_is_dir: bool,
    external_drag_count: usize,
    confirm_open: bool,
    confirm_action: Option<ConfirmAction>,
    context_menu_open: bool,
    context_menu_position: Point<Pixels>,
    context_menu_path: Option<PathBuf>,
    context_menu_is_dir: bool,
    _subscriptions: Vec<Subscription>,
    needs_focus_restore: bool,
    needs_initial_focus: bool,
    background_image: Option<PathBuf>,
}

#[derive(Clone)]
enum ConfirmAction {
    Move { src: PathBuf, dst: PathBuf },
    Delete { path: PathBuf, is_dir: bool },
}

impl StartWindow {
    fn is_image_path(path: &PathBuf) -> bool {
        match path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()) {
            Some(ext) => matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif"),
            None => false,
        }
    }
    fn is_markdown_path(path: &PathBuf) -> bool {
        match path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()) {
            Some(ext) => matches!(ext.as_str(), "md" | "markdown"),
            None => false,
        }
    }

    fn open_file_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if Self::is_image_path(&path) {
            self.image_viewer.update(cx, |viewer, cx| {
                viewer.open_image(path.clone(), cx);
            });
            if !self.open_tabs.iter().any(|p| p == &path) {
                self.open_tabs.push(path.clone());
            }
            self.active_tab = Some(path);
            cx.notify();
        } else if Self::is_markdown_path(&path) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.markdown_viewer.update(cx, |viewer, cx| {
                    viewer.set_content(content, cx);
                });
                if !self.open_tabs.iter().any(|p| p == &path) {
                    self.open_tabs.push(path.clone());
                }
                self.active_tab = Some(path);
                cx.notify();
            }
        } else if let Ok(content) = std::fs::read_to_string(&path) {
            self.editor.update(cx, |editor, cx| {
                editor.open_file(path.clone(), content, cx);
            });
            if !self.open_tabs.iter().any(|p| p == &path) {
                self.open_tabs.push(path.clone());
            }
            self.active_tab = Some(path);
            cx.notify();
        }
    }

    fn close_tab(&mut self, path: &PathBuf, cx: &mut Context<Self>) {
        let was_active = self.active_tab.as_ref() == Some(path);
        self.open_tabs.retain(|p| p != path);
        if was_active {
            if let Some(next_path) = self.open_tabs.last().cloned() {
                self.open_file_path(next_path, cx);
            } else {
                self.active_tab = None;
                self.editor.update(cx, |editor, cx| {
                    editor.set_content(String::new(), cx);
                });
            }
        }
        cx.notify();
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = &self.active_tab {
            let content = self.editor.read(cx).core.content.to_string();
            if let Err(e) = std::fs::write(path, content) {
                println!("Failed to save file: {}", e);
            } else {
                if let Some(git_panel) = self.tool_panel.read(cx).git_panel() {
                    git_panel.update(cx, |panel, _| panel.refresh());
                }
            }
        }
    }

    fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.active_tab.clone() {
            self.close_tab(&path, cx);
        }
    }

    fn request_confirm(&mut self, action: ConfirmAction, cx: &mut Context<Self>) {
        self.confirm_action = Some(action);
        self.confirm_open = true;
        cx.notify();
    }

    fn cancel_confirm(&mut self, cx: &mut Context<Self>) {
        self.confirm_open = false;
        self.confirm_action = None;
        cx.notify();
    }

    fn apply_confirm(&mut self, cx: &mut Context<Self>) {
        let action = self.confirm_action.take();
        self.confirm_open = false;
        if let Some(action) = action {
            match action {
                ConfirmAction::Move { src, dst } => {
                    match std::fs::rename(&src, &dst) {
                        Ok(_) => {
                            if let Some(index) = self.open_tabs.iter().position(|p| p == &src) {
                                self.open_tabs[index] = dst.clone();
                            }
                            if self.active_tab.as_ref() == Some(&src) {
                                self.active_tab = Some(dst);
                            }
                            let file_tree = self.file_tree.clone();
                            file_tree.update(cx, |tree, cx| {
                                tree.refresh();
                                cx.notify();
                            });
                        }
                        Err(err) => {
                            println!("Move failed: {:?} -> {:?}, {}", src, dst, err);
                        }
                    }
                }
                ConfirmAction::Delete { path, is_dir } => {
                    let result = if is_dir {
                        std::fs::remove_dir_all(&path)
                    } else {
                        std::fs::remove_file(&path)
                    };
                    match result {
                        Ok(_) => {
                            self.open_tabs.retain(|p| p != &path);
                            if self.active_tab.as_ref() == Some(&path) {
                                self.active_tab = self.open_tabs.last().cloned();
                                if let Some(next) = self.active_tab.clone() {
                                    self.open_file_path(next, cx);
                                } else {
                                    self.editor.update(cx, |editor, cx| {
                                        editor.set_content(String::new(), cx);
                                    });
                                }
                            }
                            let file_tree = self.file_tree.clone();
                            file_tree.update(cx, |tree, cx| {
                                tree.refresh();
                                cx.notify();
                            });
                        }
                        Err(err) => {
                            println!("Delete failed: {:?}, {}", path, err);
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    fn show_command_palette(&mut self, _: &ShowCommandPalette, window: &mut Window, cx: &mut Context<Self>) {
        let commands = self.plugin_manager.read(cx).command_registry.list().into_iter().cloned().collect();
        let handle = self.command_palette.read(cx).focus_handle.clone();
        handle.focus(window);
        self.command_palette.update(cx, |palette, cx| {
            palette.set_commands(commands, cx);
            palette.show(cx);
        });
    }

    fn execute_command(&mut self, command_id: &str, cx: &mut Context<Self>) {
        match command_id {
            "file_tree.toggle" => {
                self.file_tree_visible = !self.file_tree_visible;
                cx.notify();
            }
            "core.undo" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_undo(cx);
                });
            }
            "core.redo" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_redo(cx);
                });
            }
            "core.cut" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_cut(cx);
                });
            }
            "core.copy" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_copy(cx);
                });
            }
            "core.paste" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_paste(cx);
                });
            }
            "core.select_all" => {
                self.editor.update(cx, |editor, cx| {
                    editor.perform_select_all(cx);
                });
            }
            "core.save" => {
                self.save_file(cx);
            }
            "core.close" => {
                self.close_active_tab(cx);
            }
            "core.exit" => {
                std::process::exit(0);
            }
            "view.set_background" => {
                let executor = cx.background_executor().clone();
                cx.spawn(move |view: WeakEntity<StartWindow>, cx: &mut AsyncApp| {
                    let mut cx = cx.clone();
                    let executor = executor.clone();
                    async move {
                        let task = rfd::AsyncFileDialog::new()
                            .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
                            .pick_file();
                        
                        if let Some(file) = task.await {
                            let path = file.path().to_path_buf();
                            
                            // Optimization: Resize large images to reduce memory usage
                            // Offload to background thread to avoid freezing UI
                            let final_path = executor.spawn(async move {
                                if let Ok(img) = image::open(&path) {
                                    let (width, height) = img.dimensions();
                                    if width > 1920 || height > 1080 {
                                        // Use thumbnail for faster downscaling (integer scaling + bilinear)
                                        let resized = img.thumbnail(1920, 1080);
                                        
                                        // Use unique filename to avoid Windows file locking issues
                                        let timestamp = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_nanos();
                                        let temp_path = std::env::temp_dir().join(format!("tiecode_bg_cache_{}.jpg", timestamp));
                                        
                                        if let Ok(_) = resized.save(&temp_path) {
                                            temp_path
                                        } else {
                                            path
                                        }
                                    } else {
                                        path
                                    }
                                } else {
                                    path
                                }
                            }).await;

                            view.update(&mut cx, |this: &mut StartWindow, cx: &mut Context<StartWindow>| {
                                this.background_image = Some(final_path);
                                this.file_tree.update(cx, |tree: &mut FileTree, cx: &mut Context<FileTree>| {
                                    tree.set_transparent(true, cx);
                                });
                                cx.notify();
                            }).ok();
                        }
                    }
                }).detach();
            }
            _ => {
                println!("Executing command: {}", command_id);
                // Future: Delegate to plugin manager
            }
        }
    }
}

impl Render for StartWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_bg = self.background_image.is_some();
        let alpha = if has_bg { 0xcc } else { 0xff };
        let tabs_bar_bg = if has_bg { rgba(0x1f242800 | alpha) } else { rgb(0xff1f2428) };
        let tab_active_bg = if has_bg { rgba(0x2d353b00 | alpha) } else { rgb(0xff2d353b) };
        let title_bar_bg = if has_bg { rgba(0x232a2e00 | alpha) } else { rgb(0xff232a2e) };
        let main_content_bg = if has_bg { rgba(0x2d353b00 | alpha) } else { rgb(0xff2d353b) };
        let file_tree_bg = if has_bg { rgba(0x25252600 | alpha) } else { rgb(0xff252526) };
        
        let view = cx.entity();
        let view_for_focus = view.clone();
        let file_tree_view = self.file_tree.read(cx);
        let file_tree = self.file_tree.clone();
        let _file_tree_for_drop = file_tree.clone();
        let is_dragging = file_tree_view.is_dragging();
        let drag_source = file_tree_view.drag_source();
        let mouse_position = file_tree_view.mouse_position();
        let context_menu_path = self.context_menu_path.clone();
        let context_menu_is_dir = self.context_menu_is_dir;
        let context_menu_position = self.context_menu_position;
        let confirm_action = self.confirm_action.clone();
        let confirm_open = self.confirm_open;
        let view_for_confirm = view.clone();
        let view_for_cancel = view_for_confirm.clone();
        let view_for_dismiss = view_for_confirm.clone();
        let open_tabs = self.open_tabs.clone();
        let active_tab = self.active_tab.clone();
        let view_for_menu = view.clone();
        let external_drag_position = self.external_drag_position;
        let external_drag_primary = self.external_drag_primary.clone();
        let external_drag_is_dir = self.external_drag_is_dir;
        let external_drag_count = self.external_drag_count;
        let show_external_drag = cx.has_active_drag() && external_drag_primary.is_some();
        let file_tree_visible = self.file_tree_visible;

        let mut tabs_bar = div()
            .w_full()
            .h(px(28.0))
            .flex()
            .items_center()
            .bg(tabs_bar_bg)
            .border_b_1()
            .border_color(rgb(0xff3c474d))
            .px(px(6.0));

        for path in open_tabs {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let is_active = active_tab.as_ref().map(|p| p == &path).unwrap_or(false);
            let view_for_tab = view.clone();
            let view_for_close = view_for_tab.clone();
            let path_clone = path.clone();
            let path_for_close = path.clone();
            let tab = div()
                .mr(px(4.0))
                .px(px(10.0))
                .py(px(4.0))
                .rounded_md()
                .cursor_pointer()
                .text_size(px(12.0))
                .text_color(if is_active {
                    rgb(0xffe6e0d9)
                } else {
                    rgb(0xffa9b1b6)
                })
                .bg(if is_active {
                    tab_active_bg
                } else {
                    rgba(0x00000000)
                })
                .hover(|s| s.bg(rgba(0xffffff12)))
                .flex()
                .items_center()
                .child(label)
                .child(
                    div()
                        .ml(px(6.0))
                        .text_size(px(12.0))
                        .text_color(rgb(0xff8b949e))
                        .hover(|s| s.text_color(rgb(0xffe6e0d9)))
                        .child("×")
                        .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                            cx.stop_propagation();
                            view_for_close.update(cx, |this, cx| {
                                this.close_tab(&path_for_close, cx);
                            });
                        }),
                )
                .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                    view_for_tab.update(cx, |this, cx| {
                        this.open_file_path(path_clone.clone(), cx);
                    });
                });
            tabs_bar = tabs_bar.child(tab);
        }

        let (confirm_title, confirm_body) = match &confirm_action {
            Some(ConfirmAction::Move { src, dst }) => (
                "确认移动".to_string(),
                div()
                    .flex()
                    .flex_col()
                    .child("将")
                    .child(
                        div()
                            .mt(px(4.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child(src.to_string_lossy().to_string()),
                    )
                    .child(div().mt(px(6.0)).child("移动到"))
                    .child(
                        div()
                            .mt(px(4.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child(dst.to_string_lossy().to_string()),
                    )
                    .into_any_element(),
            ),
            Some(ConfirmAction::Delete { path, is_dir }) => (
                "确认删除".to_string(),
                div()
                    .flex()
                    .flex_col()
                    .child(format!(
                        "确定删除{}：",
                        if *is_dir { "文件夹" } else { "文件" }
                    ))
                    .child(
                        div()
                            .mt(px(6.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child(path.to_string_lossy().to_string()),
                    )
                    .into_any_element(),
            ),
            None => ("确认".to_string(), div().into_any_element()),
        };

        let content = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .on_children_prepainted(move |_, window, cx| {
                view_for_focus.update(cx, |this, cx| {
                    if (this.needs_focus_restore || this.needs_initial_focus)
                        && !this.command_palette.read(cx).is_visible()
                    {
                        this.needs_focus_restore = false;
                        this.needs_initial_focus = false;
                        let handle = this.editor.read(cx).focus_handle.clone();
                        handle.focus(window);
                    }
                });
            })
            .on_drag_move(cx.listener(|this, event: &DragMoveEvent<ExternalPaths>, _window, cx| {
                let paths = event.drag(cx).paths();
                this.external_drag_position = event.event.position;
                this.external_drag_primary = paths.first().cloned();
                this.external_drag_count = paths.len();
                this.external_drag_is_dir = paths.first().map(|p| p.is_dir()).unwrap_or(false);
                cx.notify();
            }))
            .on_drop(cx.listener(move |this, paths: &ExternalPaths, _window, cx| {
                this.external_drag_primary = None;
                this.external_drag_count = 0;
                this.external_drag_is_dir = false;
                if let Some(path) = paths.paths().first() {
                     if path.is_dir() {
                         println!("Dropping folder: {:?}", path);
                         this.file_tree.update(cx, |tree, cx| {
                             tree.set_root_path(path.clone(), cx);
                         });
                         
                         let path_clone = path.clone();
                         this.tool_panel.update(cx, |panel, cx| {
                            if let Some(git_panel) = panel.git_panel() {
                                git_panel.update(cx, |gp, cx| {
                                    gp.set_repo_root(path_clone, cx);
                                });
                            }
                         });
                     }
                }
                cx.notify();
            }))
            .child(
                div()
                    .w_full()
                    .h(px(30.0))
                    .bg(title_bar_bg)
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
                    .bg(main_content_bg)
                    .child(
                        if file_tree_visible {
                            div()
                                .w(px(260.0))
                                .h_full()
                                .border_r_1()
                                .border_color(rgb(0xff3c474d))
                                .bg(file_tree_bg)
                                .child(self.tool_panel.clone())
                        } else {
                            div()
                        }
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .h_full()
                            .child(tabs_bar)
                            .child({
                                let is_image = self.active_tab.as_ref().map(|p| Self::is_image_path(p)).unwrap_or(false);
                                if is_image {
                                    div().flex_1().child(self.image_viewer.clone())
                                } else {
                                    let is_md = self.active_tab.as_ref().map(|p| Self::is_markdown_path(p)).unwrap_or(false);
                                    if is_md {
                                        div().flex_1().child(self.markdown_viewer.clone())
                                    } else {
                                        div().flex_1().child(self.editor.clone())
                                    }
                                }
                            })
                    ),
            )
            .child(self.status_bar.clone())
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
            .child(
                modal()
                    .open(confirm_open)
                    .title(confirm_title)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child(confirm_body),
                    )
                    .footer(
                        div()
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded_md()
                                    .bg(rgb(0xff3c474d))
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .mr(px(8.0))
                                    .child("取消")
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        view_for_cancel.update(cx, |this, cx| {
                                            this.cancel_confirm(cx);
                                        });
                                    }),
                            )
                            .child({
                                let view_for_confirm = view.clone();
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded_md()
                                    .bg(rgb(0xff2d6cdf))
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xffffffff))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0xff3b7bff)))
                                    .child("确定")
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        view_for_confirm.update(cx, |this, cx| {
                                            this.apply_confirm(cx);
                                        });
                                    })
                            }),
                    )
                    .on_dismiss(move |_window, cx| {
                        view_for_dismiss.update(cx, |this, cx| {
                            this.cancel_confirm(cx);
                        });
                    }),
            )
            .child(if show_external_drag {
                let name = external_drag_primary
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "文件".to_string());
                let title = if external_drag_is_dir {
                    "松开打开文件夹"
                } else {
                    "仅支持拖拽文件夹"
                };
                let subtitle = if external_drag_count > 1 {
                    format!("{} 个项目", external_drag_count)
                } else {
                    name
                };
                let icon = if external_drag_is_dir {
                    tie_svg()
                        .path("assets/icons/folder_dark.svg")
                        .size(px(36.0))
                        .original_colors(true)
                        .into_any_element()
                } else {
                    tie_svg()
                        .path("assets/icons/anyType_dark.svg")
                        .size(px(36.0))
                        .original_colors(true)
                        .into_any_element()
                };
                div()
                    .absolute()
                    .top(external_drag_position.y + px(12.0))
                    .left(external_drag_position.x + px(12.0))
                    .bg(rgba(0x1f2428e6))
                    .border_1()
                    .border_color(rgba(0xffffff24))
                    .rounded_md()
                    .px(px(10.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .child(div().mr(px(8.0)).child(icon))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .mt(px(2.0))
                                    .text_size(px(11.0))
                                    .text_color(rgb(0xffa9b1b6))
                                    .child(subtitle),
                            ),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .child(self.command_palette.clone())
            .on_action(cx.listener(Self::show_command_palette))
            /*
            .child(
                modal()
                    .open(self.show_modal)
                    .title("弹窗")
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child("点击了按钮，弹窗已打开"),
                    )
                    .on_dismiss(move |_window, cx| {
                        view_for_modal.update(cx, |this, cx| {
                            this.show_modal = false;
                            cx.notify();
                        });
                    }),
            )
            */
            .child(
                popover()
                    .open(self.context_menu_open)
                    .position(context_menu_position)
                    .w(px(180.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child({
                                let view = view_for_menu.clone();
                                let file_tree = file_tree.clone();
                                let path = context_menu_path.clone();
                                let label = if context_menu_is_dir { "展开/折叠" } else { "打开" };
                                div()
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .child(label)
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        if let Some(path) = path.clone() {
                                            if context_menu_is_dir {
                                                file_tree.update(cx, |tree, cx| {
                                                    tree.toggle_dir(path.clone(), cx);
                                                });
                                            } else {
                                                file_tree.update(cx, |_, cx| {
                                                    cx.emit(FileTreeEvent::OpenFile(path.clone()));
                                                });
                                            }
                                        }
                                        view.update(cx, |this, cx| {
                                            this.context_menu_open = false;
                                            this.context_menu_path = None;
                                            cx.notify();
                                        });
                                    })
                            })
                            .child({
                                let view = view_for_menu.clone();
                                let file_tree = file_tree.clone();
                                let path = context_menu_path.clone();
                                div()
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .child("新建文件")
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        if let Some(path) = path.clone() {
                                            file_tree.update(cx, |tree, cx| {
                                                tree.begin_inline_create(
                                                    path.clone(),
                                                    context_menu_is_dir,
                                                    false,
                                                    cx,
                                                );
                                            });
                                            file_tree.read(cx).focus(window);
                                        }
                                        view.update(cx, |this, cx| {
                                            this.context_menu_open = false;
                                            this.context_menu_path = None;
                                            cx.notify();
                                        });
                                    })
                            })
                            .child({
                                let view = view_for_menu.clone();
                                let file_tree = file_tree.clone();
                                let path = context_menu_path.clone();
                                div()
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .child("新建文件夹")
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        if let Some(path) = path.clone() {
                                            file_tree.update(cx, |tree, cx| {
                                                tree.begin_inline_create(
                                                    path.clone(),
                                                    context_menu_is_dir,
                                                    true,
                                                    cx,
                                                );
                                            });
                                            file_tree.read(cx).focus(window);
                                        }
                                        view.update(cx, |this, cx| {
                                            this.context_menu_open = false;
                                            this.context_menu_path = None;
                                            cx.notify();
                                        });
                                    })
                            })
                            .child({
                                let view = view_for_menu.clone();
                                let path = context_menu_path.clone();
                                div()
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .child("复制路径")
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        if let Some(path) = path.clone() {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                path.to_string_lossy().to_string(),
                                            ));
                                        }
                                        view.update(cx, |this, cx| {
                                            this.context_menu_open = false;
                                            this.context_menu_path = None;
                                            cx.notify();
                                        });
                                    })
                            })
                            .child({
                                let view = view_for_menu.clone();
                                let path = context_menu_path.clone();
                                div()
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .text_size(px(13.0))
                                    .text_color(rgb(0xffe6e0d9))
                                    .hover(|s| s.bg(rgba(0xffffff12)))
                                    .child("删除")
                                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                        if let Some(path) = path.clone() {
                                            view.update(cx, |this, cx| {
                                                this.request_confirm(
                                                    ConfirmAction::Delete {
                                                        path: path.clone(),
                                                        is_dir: context_menu_is_dir,
                                                    },
                                                    cx,
                                                );
                                                this.context_menu_open = false;
                                                this.context_menu_path = None;
                                                cx.notify();
                                            });
                                        }
                                    })
                            }),
                    )
                    .on_dismiss(move |_window, cx| {
                        view_for_menu.update(cx, |this, cx| {
                            this.context_menu_open = false;
                            this.context_menu_path = None;
                            cx.notify();
                        });
                    }),
            );

        if let Some(bg_path) = self.background_image.clone() {
            div()
                .size_full()
                .child(
                    img(bg_path)
                        .absolute()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                )
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .bg(rgba(0x00000080))
                )
                .child(content)
                .into_any_element()
        } else {
            content.bg(rgb(0xFFFFFFFF)).into_any_element()
        }
    }
}
