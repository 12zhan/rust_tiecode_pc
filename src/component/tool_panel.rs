use gpui::*;
use std::path::PathBuf;
use crate::component::file_tree::FileTree;
use crate::component::tie_svg::tie_svg;

#[derive(Clone)]
pub struct ToolEntry {
    pub id: String,
    pub label: String,
    pub icon: Option<PathBuf>,
    pub builtin_explorer: bool,
}

pub struct ToolPanel {
    entries: Vec<ToolEntry>,
    selected: usize,
    file_tree: Entity<FileTree>,
    git_panel: Option<Entity<crate::component::git_panel::GitPanel>>,
}

impl ToolPanel {
    pub fn new(file_tree: Entity<FileTree>, _cx: &mut Context<Self>) -> Self {
        let mut entries = Vec::new();
        entries.push(ToolEntry {
            id: "explorer".to_string(),
            label: "文件".to_string(),
            icon: Some(PathBuf::from("assets/icons/folder_dark.svg")),
            builtin_explorer: true,
        });
        Self {
            entries,
            selected: 0,
            file_tree,
            git_panel: None,
        }
    }

    pub fn add_tool_page(&mut self, id: impl Into<String>, label: impl Into<String>, icon: Option<PathBuf>) {
        self.entries.push(ToolEntry {
            id: id.into(),
            label: label.into(),
            icon,
            builtin_explorer: false,
        });
    }

    pub fn attach_git_panel(&mut self, git_panel: Entity<crate::component::git_panel::GitPanel>) {
        self.git_panel = Some(git_panel);
    }

    pub fn git_panel(&self) -> Option<Entity<crate::component::git_panel::GitPanel>> {
        self.git_panel.clone()
    }
}

impl Render for ToolPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.entries.clone();
        let selected = self.selected;
        let panel = cx.entity();
        let header = div()
            .w_full()
            .h(px(32.0))
            .bg(rgb(0xff232a2e))
            .border_b_1()
            .border_color(rgb(0xff3c474d))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0));
        let mut header = header;
        for (i, e) in entries.iter().enumerate() {
            let icon_elem = if e.id == "git" {
                tie_svg()
                    .path(SharedString::from("assets/git.svg"))
                    .size(px(18.0))
                    .original_colors(false)
                    .text_color(rgb(0xfff14e32))
            } else if let Some(path) = &e.icon {
                tie_svg()
                    .path(path.to_string_lossy().to_string())
                    .size(px(18.0))
                    .original_colors(true)
            } else {
                tie_svg()
                    .path("assets/icons/anyType_dark.svg")
                    .size(px(18.0))
                    .original_colors(true)
            };
            let idx = i;
            let panel_for_click = panel.clone();
            header = header.child(
                div()
                    .p(px(6.0))
                    .rounded_md()
                    .cursor_pointer()
                    .bg(if selected == idx { rgb(0xff2d353b) } else { rgba(0x00000000) })
                    .hover(|s| s.bg(rgba(0xffffff12)))
                    .child(icon_elem)
                    .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                        panel_for_click.update(cx, |this, cx_inner| {
                            this.selected = idx;
                            cx_inner.notify();
                        });
                    }),
            );
        }
        let body: AnyElement = {
            if entries.get(selected).map(|e| e.builtin_explorer).unwrap_or(false) {
                self.file_tree.clone().into_any_element()
            } else {
                let label = entries.get(selected).map(|e| e.label.clone()).unwrap_or("工具".to_string());
                if entries.get(selected).map(|e| e.id.as_str() == "git").unwrap_or(false) {
                    if let Some(panel) = &self.git_panel {
                        panel.clone().into_any_element()
                    } else {
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(0xffa9b1b6))
                            .child("Git 工具未初始化")
                            .into_any_element()
                    }
                } else {
                    div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .p(px(12.0))
                    .text_size(px(13.0))
                    .text_color(rgb(0xffe6e0d9))
                    .child(
                        div()
                            .text_color(rgb(0xffa9b1b6))
                            .child("资源管理器"),
                    )
                    .child(
                        div()
                            .mt(px(8.0))
                            .text_color(rgb(0xffe6e0d9))
                            .child(format!("工具页面：{}", label)),
                    )
                        .into_any_element()
                }
            }
        };
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0xff252526))
            .child(header)
            .child(body)
    }
}
