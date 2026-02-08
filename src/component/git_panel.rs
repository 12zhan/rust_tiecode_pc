use gpui::*;
use std::path::PathBuf;
use git2::{Repository, Status, StatusOptions, IndexAddOption};
use super::tie_svg::tie_svg;
use super::file_tree::file_icon;
use crate::component::mod_rs_helpers::{byte_index_to_utf16, utf16_index_to_byte};

#[derive(Clone)]
pub struct GitChange {
    pub path: String,
    pub status: String,
}

pub struct GitPanel {
    repo_root: Option<PathBuf>,
    focus_handle: FocusHandle,
    commit_message: String,
    commit_cursor: usize,
    commit_message_marked_range: Option<std::ops::Range<usize>>,
    input_bounds: Option<Bounds<Pixels>>,
    changes: Vec<GitChange>,
    branch: String,
    branches: Vec<String>,
    branch_list_state: ListState,
    ahead: i32,
    behind: i32,
    list_state: ListState,
    is_repo: bool,
}

impl GitPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            repo_root: None,
            focus_handle: cx.focus_handle(),
            commit_message: String::new(),
            commit_cursor: 0,
            commit_message_marked_range: None,
            input_bounds: None,
            changes: Vec::new(),
            branch: String::new(),
            branches: Vec::new(),
            branch_list_state: ListState::new(0, ListAlignment::Top, px(24.0)),
            ahead: 0,
            behind: 0,
            list_state: ListState::new(0, ListAlignment::Top, px(24.0)),
            is_repo: false,
        };
        this.refresh();
        this
    }

    pub fn set_repo_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.repo_root = Some(path);
        self.refresh();
        cx.notify();
    }

    fn init_repo(&mut self) {
        if let Some(root) = &self.repo_root {
            if Repository::init(root).is_ok() {
                self.refresh();
            }
        }
    }

    fn push_changes(&mut self) {
        if let Some(root) = &self.repo_root {
            // TODO: Use git2 credentials for push if possible, or keep shell for now
            let _ = std::process::Command::new("git")
                .arg("push")
                .current_dir(root)
                .spawn();
        }
    }

    fn refresh(&mut self) {
        let root = match &self.repo_root {
            Some(r) => r,
            None => {
                self.is_repo = false;
                self.branch = String::new();
                self.branches.clear();
                self.changes.clear();
                return;
            }
        };

        let repo = match Repository::open(root) {
            Ok(r) => {
                self.is_repo = true;
                r
            },
            Err(_) => {
                self.is_repo = false;
                self.branch = "No Git Repo".to_string();
                self.branches.clear();
                return;
            }
        };

        // branch
        self.branch = match repo.head() {
            Ok(head) => head.shorthand().unwrap_or("DETACHED").to_string(),
            Err(_) => "No HEAD".to_string(),
        };

        // branches list
        self.branches.clear();
        if let Ok(branches) = repo.branches(None) {
            for b in branches {
                if let Ok((branch, _)) = b {
                    if let Ok(Some(name)) = branch.name() {
                        self.branches.push(name.to_string());
                    }
                }
            }
        }
        self.branches.sort();
        self.branch_list_state = ListState::new(self.branches.len(), ListAlignment::Top, px(20.0));

        // ahead/behind
        self.ahead = 0;
        self.behind = 0;
        if let Ok(head) = repo.head() {
            if let Ok(upstream) = repo.branch_upstream_name(head.name().unwrap_or("")) {
                if let Some(upstream_str) = upstream.as_str() {
                     if let (Ok(local_oid), Ok(upstream_oid)) = (
                         repo.refname_to_id(head.name().unwrap_or("")),
                         repo.refname_to_id(upstream_str)
                     ) {
                         if let Ok((a, b)) = repo.graph_ahead_behind(local_oid, upstream_oid) {
                             self.ahead = a as i32;
                             self.behind = b as i32;
                         }
                     }
                }
            }
        }

        // status
        self.changes.clear();
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        
        if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
            for entry in statuses.iter() {
                let path = entry.path().unwrap_or("").to_string();
                let status = entry.status();
                let status_str = format_status(status);
                self.changes.push(GitChange { path, status: status_str });
            }
        }
        self.list_state = ListState::new(self.changes.len(), ListAlignment::Top, px(24.0));
    }

    fn commit_all(&mut self) {
        if self.commit_message.trim().is_empty() {
            return;
        }
        
        let root = match &self.repo_root {
            Some(r) => r,
            None => return,
        };
        
        let repo = match Repository::open(root) {
            Ok(r) => r,
            Err(_) => return,
        };
        
        let mut index = match repo.index() {
            Ok(i) => i,
            Err(_) => return,
        };
        
        if index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None).is_err() {
            return;
        }
        if index.write().is_err() { return; }
        
        let tree_id = match index.write_tree() {
            Ok(id) => id,
            Err(_) => return,
        };
        let tree = match repo.find_tree(tree_id) {
            Ok(t) => t,
            Err(_) => return,
        };
        
        let signature = match repo.signature() {
            Ok(s) => s,
            Err(_) => {
                 git2::Signature::now("TieCode User", "user@tiecode.dev").unwrap()
            }
        };
        
        let parent_commit = if let Ok(head) = repo.head() {
            if let Ok(target) = head.resolve() {
                target.peel_to_commit().ok()
            } else { None }
        } else { None };

        let parents = if let Some(ref p) = parent_commit {
            vec![p]
        } else {
            vec![]
        };

        let _ = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &self.commit_message,
            &tree,
            &parents,
        );
        
        self.commit_message.clear();
        self.commit_cursor = 0;
        self.refresh();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key == "enter" {
            self.commit_all();
            cx.notify();
            return;
        }
        if key == "backspace" {
            if self.commit_cursor > 0 && !self.commit_message.is_empty() {
                let prev = prev_char_boundary(&self.commit_message, self.commit_cursor);
                self.commit_message
                    .replace_range(prev..self.commit_cursor, "");
                self.commit_cursor = prev;
                cx.notify();
            }
            return;
        }
        if key == "left" {
            self.commit_cursor = prev_char_boundary(&self.commit_message, self.commit_cursor);
            cx.notify();
            return;
        }
        if key == "right" {
            self.commit_cursor = next_char_boundary(&self.commit_message, self.commit_cursor);
            cx.notify();
            return;
        }
    }
}

fn format_status(s: Status) -> String {
    if s.contains(Status::INDEX_NEW) { "A ".to_string() }
    else if s.contains(Status::INDEX_MODIFIED) { "M ".to_string() }
    else if s.contains(Status::WT_NEW) { "? ".to_string() }
    else if s.contains(Status::WT_MODIFIED) { " M".to_string() }
    else if s.contains(Status::INDEX_DELETED) { "D ".to_string() }
    else if s.contains(Status::WT_DELETED) { " D".to_string() }
    else { "  ".to_string() }
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

impl Render for GitPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_bg = rgb(0xff252526);
        let _theme_border = rgb(0xff3c474d); // Border color
        let theme_text = rgb(0xffcccccc); // Main text
        let theme_muted = rgb(0xff858585); // Muted text
        let theme_hover = rgb(0xff2a2d2e); // List item hover
        let theme_header_bg = rgb(0xff252526); // Header background
        let panel = cx.entity();
        let focus = self.focus_handle.clone();
        let changes = self.changes.clone();
        let branch = self.branch.clone();
        let branches = self.branches.clone();
        let ahead = self.ahead;
        let behind = self.behind;
        
        if self.repo_root.is_none() {
             return div()
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .bg(theme_bg)
                .child(
                    div()
                        .text_color(theme_muted)
                        .text_size(px(13.0))
                        .child("没有打开的文件夹")
                )
                .into_any_element();
        }

        if !self.is_repo {
            return div()
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .bg(theme_bg)
                .child(
                    div()
                        .px(px(12.0))
                        .py(px(6.0))
                        .rounded_md()
                        .bg(rgb(0xff238636)) // GitHub green
                        .text_color(rgb(0xffffffff))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xff2ea043)))
                        .child("初始化仓库")
                        .on_mouse_down(MouseButton::Left, move |_, _w, cx| {
                            panel.update(cx, |this, _cx| {
                                this.init_repo();
                            });
                        }),
                )
                .into_any_element();
        }

        let header = div()
            .w_full()
            .flex()
            .flex_col()
            .bg(theme_header_bg)
            .child(
                div()
                    .w_full()
                    .h(px(36.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(theme_text)
                            .text_size(px(11.0))
                            .font_weight(FontWeight::BOLD)
                            .child("源代码管理".to_uppercase())
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .items_center()
                            .child(
                                div()
                                    .text_color(theme_text)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(rgb(0xffffffff)))
                                    .child(
                                        tie_svg()
                                            .path("assets/icons/sync.svg")
                                            .size(px(14.0))
                                            .text_color(theme_text)
                                            .into_any_element()
                                    )
                                    .on_mouse_down(MouseButton::Left, {
                                        let panel = panel.clone();
                                        move |_, _w, cx| {
                                            panel.update(cx, |this, _| {
                                                this.push_changes();
                                            });
                                        }
                                    })
                            )
                            .child(
                                div()
                                    .text_color(theme_muted)
                                    .text_size(px(11.0))
                                    .child(format!("{}  ↑{} ↓{}", branch, ahead, behind)),
                            )
                    )
            );

        let branch_list = if !branches.is_empty() {
             div()
                .w_full()
                .flex()
                .flex_col()
                .mt(px(8.0))
                .child(
                    div()
                        .px(px(16.0))
                        .pb(px(4.0))
                        .text_color(theme_muted)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .child("分支")
                )
                .child(
                    div()
                        .w_full()
                        .h(px(120.0))
                        .child(
                            list(
                                self.branch_list_state.clone(),
                                move |index, _window, _cx| {
                                    if index >= branches.len() { return div().into_any_element(); }
                                    let b = &branches[index];
                                    let is_current = b == &branch;
                                    div()
                                        .w_full()
                                        .h(px(24.0))
                                        .px(px(16.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme_hover))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_color(if is_current { rgb(0xff2ea043) } else { theme_text })
                                                .text_size(px(13.0))
                                                .child(b.clone())
                                        )
                                        .child(
                                            if is_current {
                                                div().child(
                                                    tie_svg()
                                                        .path("assets/icons/check.svg")
                                                        .size(px(12.0))
                                                        .text_color(rgb(0xff2ea043))
                                                        .into_any_element()
                                                )
                                            } else {
                                                div()
                                            }
                                        )
                                        .into_any_element()
                                }
                            )
                            .w_full()
                            .h_full()
                        )
                )
        } else {
            div()
        };

        let commit_input = {
            let msg = if self.commit_message.is_empty() {
                "消息 (Ctrl+Enter 提交)".to_string()
            } else {
                self.commit_message.clone()
            };
            div()
                .w_full()
                .p(px(16.0))
                .child(
                    div()
                        .w_full()
                        .bg(rgb(0xff3c3c3c))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0xff454545))
                        .px(px(10.0))
                        .py(px(8.0))
                        .text_color(if self.commit_message.is_empty() { theme_muted } else { theme_text })
                        .text_size(px(13.0))
                        .child(msg),
                )
                .child(
                    canvas(
                        |bounds, _window, _cx| bounds,
                        {
                            let panel = panel.clone();
                            move |bounds, _layout, window, cx| {
                                panel.update(cx, |this, _| {
                                    this.input_bounds = Some(bounds);
                                });
                                window.handle_input(
                                    &focus,
                                    ElementInputHandler::new(bounds, panel.clone()),
                                    cx,
                                );
                                
                                // Draw cursor if focused
                                if focus.is_focused(window) {
                                    panel.update(cx, |this, _cx| {
                                        let style = window.text_style();
                                        let run = TextRun {
                                            len: this.commit_message.len(),
                                            font: style.font(),
                                            color: theme_text.into(),
                                            background_color: None,
                                            underline: None,
                                            strikethrough: None,
                                        };
                                        let line = window.text_system().shape_line(
                                            SharedString::from(this.commit_message.clone()),
                                            px(13.0),
                                            &[run],
                                            None,
                                        );
                                        
                                        let cursor_idx = this.commit_cursor.min(this.commit_message.len());
                                        let x = line.x_for_index(cursor_idx) + px(10.0) + bounds.left();
                                        let y_start = bounds.top() + px(8.0);
                                        let height = px(16.0);
                                        
                                        window.paint_quad(fill(
                                            Bounds::new(point(x, y_start), size(px(1.5), height)),
                                            rgb(0xff007fd4),
                                        ));
                                    });
                                }
                            }
                        },
                    )
                    .absolute()
                    .top(px(16.0))
                    .left(px(16.0))
                    .right(px(16.0))
                    .h(px(32.0)),
                )
                .child(
                    div()
                        .mt(px(8.0))
                        .w_full()
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .px(px(14.0))
                                .py(px(6.0))
                                .rounded_md()
                                .bg(rgb(0xff238636))
                                .text_color(rgb(0xffffffff))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0xff2ea043)))
                                .child("提交")
                                .on_mouse_down(MouseButton::Left, move |_, _w, cx| {
                                    panel.update(cx, |this, cx_inner| {
                                        this.commit_all();
                                        cx_inner.notify();
                                    });
                                }),
                        ),
                )
        };

        let changes_len = changes.len();
        let list_changes = list(
            self.list_state.clone(),
            move |index, _window, _cx| {
            if index >= changes.len() {
                return div().into_any_element();
            }
            let ch = &changes[index];
            let status_color = if ch.status.contains('M') {
                rgb(0xffe2c08d) // Modified (Yellowish)
            } else if ch.status.contains('A') || ch.status.contains('?') {
                rgb(0xff73c991) // Added (Greenish)
            } else if ch.status.contains('D') {
                rgb(0xfff14c4c) // Deleted (Red)
            } else {
                theme_muted
            };

            div()
                .w_full()
                .h(px(24.0))
                .px(px(16.0))
                .flex()
                .justify_between()
                .items_center()
                .bg(theme_bg)
                .text_color(theme_text)
                .cursor_pointer()
                .hover(|s| s.bg(theme_hover))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w(px(16.0))
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .text_color(status_color)
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child(ch.status.trim().chars().next().unwrap_or(' ').to_string())
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(file_icon(&ch.path))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .child(ch.path.clone())
                                )
                        ),
                )
                .into_any_element()
        })
        .w_full()
        .flex_1(); // Use flex_1 to fill remaining space

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme_bg)
            .child(header)
            .child(commit_input)
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_color(theme_muted)
                            .text_size(px(11.0))
                            .font_weight(FontWeight::BOLD)
                            .child("更改")
                    )
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .rounded_md()
                            .bg(rgb(0xff3c3c3c))
                            .text_color(theme_text)
                            .text_size(px(11.0))
                            .child(changes_len.to_string())
                    )
            )
            .child(list_changes)
            .child(branch_list)
            .on_key_down(cx.listener(|this: &mut GitPanel, event: &KeyDownEvent, _window, cx| {
                this.on_key_down(event, _window, cx);
            }))
            .into_any_element()
    }
}

impl EntityInputHandler for GitPanel {
    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        self.commit_message_marked_range.clone()
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.commit_message_marked_range = None;
    }

    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        adjusted_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = utf16_range_to_byte_range(&self.commit_message, range_utf16);
        adjusted_range.replace(byte_range_to_utf16_range(&self.commit_message, range.clone()));
        Some(self.commit_message[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let cursor_utf16 = byte_index_to_utf16(&self.commit_message, self.commit_cursor);
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
        let start = range_utf16
            .as_ref()
            .map(|r| utf16_index_to_byte(&self.commit_message, r.start))
            .unwrap_or(self.commit_cursor)
            .min(self.commit_message.len());
        let end = range_utf16
            .as_ref()
            .map(|r| utf16_index_to_byte(&self.commit_message, r.end))
            .unwrap_or(self.commit_cursor)
            .min(self.commit_message.len());
        self.commit_message.replace_range(start..end, new_text);
        self.commit_cursor = start + new_text.len();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
        self.commit_message_marked_range = new_selected_range_utf16;
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let input_bounds = self.input_bounds.unwrap_or(bounds);
        let style = window.text_style();
        let run = TextRun {
            len: self.commit_message.len(),
            font: style.font(),
            color: rgb(0xffe6e0d9).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window.text_system().shape_line(
            SharedString::from(self.commit_message.clone()),
            px(13.0),
            &[run],
            None,
        );
        let start_b = utf16_index_to_byte(&self.commit_message, range_utf16.start);
        let end_b = utf16_index_to_byte(&self.commit_message, range_utf16.end);
        let x0 = line.x_for_index(start_b.min(self.commit_message.len()));
        let x1 = line.x_for_index(end_b.min(self.commit_message.len()));
        Some(Bounds::from_corners(
            point(input_bounds.left() + px(10.0) + x0, input_bounds.top() + px(8.0)),
            point(input_bounds.left() + px(10.0) + x1, input_bounds.top() + px(8.0) + px(16.0)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let input_bounds = self.input_bounds?;
        let local_x = (point.x - (input_bounds.left() + px(10.0))).max(px(0.0));
        let style = window.text_style();
        let run = TextRun {
            len: self.commit_message.len(),
            font: style.font(),
            color: rgb(0xffe6e0d9).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window.text_system().shape_line(
            SharedString::from(self.commit_message.clone()),
            px(13.0),
            &[run],
            None,
        );
        let idx = line.index_for_x(local_x).unwrap_or(self.commit_message.len());
        Some(byte_index_to_utf16(&self.commit_message, idx))
    }
}

// Helper to missing range conversion (copied from mod_rs_helpers to avoid import ambiguity if needed, but we imported it)
fn utf16_range_to_byte_range(text: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let start = utf16_index_to_byte(text, range.start);
    let end = utf16_index_to_byte(text, range.end);
    start..end
}

fn byte_range_to_utf16_range(text: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let start = byte_index_to_utf16(text, range.start);
    let end = byte_index_to_utf16(text, range.end);
    start..end
}
