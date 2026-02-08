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

#[derive(Clone, Copy, PartialEq)]
enum GitPanelMode {
    Changes,
    History,
}

#[derive(Clone)]
struct CommitInfo {
    id: String,
    short_id: String,
    message: String,
    author: String,
    time: i64,
}

pub struct GitPanel {
    repo_root: Option<PathBuf>,
    focus_handle: FocusHandle,
    commit_message: String,
    commit_cursor: usize,
    commit_selection: Option<std::ops::Range<usize>>,
    drag_start_index: Option<usize>,
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
    mode: GitPanelMode,
    commits: Vec<CommitInfo>,
    history_list_state: ListState,
    selected_commit_index: Option<usize>,
    commit_changes: Vec<GitChange>,
    commit_changes_list_state: ListState,
}

impl GitPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            repo_root: None,
            focus_handle: cx.focus_handle(),
            commit_message: String::new(),
            commit_cursor: 0,
            commit_selection: None,
            drag_start_index: None,
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
            mode: GitPanelMode::Changes,
            commits: Vec::new(),
            history_list_state: ListState::new(0, ListAlignment::Top, px(50.0)),
            selected_commit_index: None,
            commit_changes: Vec::new(),
            commit_changes_list_state: ListState::new(0, ListAlignment::Top, px(24.0)),
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

    fn load_history(&mut self) {
        self.selected_commit_index = None;
        self.commits.clear();
        let root = match &self.repo_root {
            Some(r) => r,
            None => return,
        };
        let repo = match Repository::open(root) {
            Ok(r) => r,
            Err(_) => return,
        };
        
        let mut revwalk = match repo.revwalk() {
            Ok(r) => r,
            Err(_) => return,
        };
        
        if revwalk.push_head().is_err() {
            return;
        }
        revwalk.set_sorting(git2::Sort::TIME).ok();
        
        for id in revwalk.take(100) {
            if let Ok(id) = id {
                if let Ok(commit) = repo.find_commit(id) {
                    let message = commit.summary().unwrap_or("").to_string();
                    let author = commit.author().name().unwrap_or("").to_string();
                    let time = commit.time().seconds();
                    let short_id = id.to_string()[..7].to_string();
                    
                    self.commits.push(CommitInfo {
                        id: id.to_string(),
                        short_id,
                        message,
                        author,
                        time,
                    });
                }
            }
        }
        self.history_list_state = ListState::new(self.commits.len(), ListAlignment::Top, px(50.0));
    }

    fn load_commit_changes(&mut self, index: usize) {
        if index >= self.commits.len() { return; }
        let commit_info = &self.commits[index];
        let root = match &self.repo_root { Some(r) => r, None => return };
        let repo = match Repository::open(root) { Ok(r) => r, Err(_) => return };
        
        let commit_oid = match git2::Oid::from_str(&commit_info.id) { Ok(id) => id, Err(_) => return };
        let commit = match repo.find_commit(commit_oid) { Ok(c) => c, Err(_) => return };
        let tree = match commit.tree() { Ok(t) => t, Err(_) => return };
        
        let parent_tree = if commit.parent_count() > 0 {
             if let Ok(parent) = commit.parent(0) {
                 parent.tree().ok()
             } else { None }
        } else {
            None
        };
        
        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.include_typechange(true);
        
        let diff = if let Some(pt) = parent_tree {
             repo.diff_tree_to_tree(Some(&pt), Some(&tree), Some(&mut diff_opts))
        } else {
             repo.diff_tree_to_tree(None, Some(&tree), Some(&mut diff_opts))
        };
        
        self.commit_changes.clear();
        if let Ok(diff) = diff {
             for delta in diff.deltas() {
                 let status_char = match delta.status() {
                     git2::Delta::Added => "A ",
                     git2::Delta::Deleted => "D ",
                     git2::Delta::Modified => "M ",
                     git2::Delta::Renamed => "R ",
                     git2::Delta::Copied => "C ",
                     _ => "  ",
                 };
                 
                 let path = if delta.status() == git2::Delta::Deleted {
                     delta.old_file().path()
                 } else {
                     delta.new_file().path()
                 }.unwrap_or(std::path::Path::new("")).to_string_lossy().to_string();
                 
                 self.commit_changes.push(GitChange { path, status: status_char.to_string() });
             }
        }
        self.commit_changes_list_state = ListState::new(self.commit_changes.len(), ListAlignment::Top, px(24.0));
    }

    pub fn refresh(&mut self) {
        let root = match &self.repo_root {
            Some(r) => r,
            None => {
                self.is_repo = false;
                self.branch = String::new();
                self.branches.clear();
                self.changes.clear();
                self.commits.clear();
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
                self.commits.clear();
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
        
        self.load_history();
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

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key == "enter" {
            self.commit_all();
            cx.notify();
            return;
        }
        if key == "enter" {
            self.commit_message.insert(self.commit_cursor, '\n');
            self.commit_cursor += 1;
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
        if key == "up" {
            let p = self.point_for_index(self.commit_cursor, window);
            let line_height = px(13.0 * 1.4);
            let new_p = point(p.x, p.y - line_height);
            self.commit_cursor = self.index_for_point(new_p, window);
            cx.notify();
            return;
        }
        if key == "down" {
            let p = self.point_for_index(self.commit_cursor, window);
            let line_height = px(13.0 * 1.4);
            let new_p = point(p.x, p.y + line_height);
            self.commit_cursor = self.index_for_point(new_p, window);
            cx.notify();
            return;
        }
    }

    fn point_for_index(&self, index: usize, window: &Window) -> Point<Pixels> {
        let bounds = match self.input_bounds {
            Some(b) => b,
            None => return Point::default(),
        };
        
        if self.commit_message.is_empty() {
            return bounds.origin;
        }

        let style = window.text_style();
        let font_size = px(13.0);
        let text = &self.commit_message;
        let font = style.font();
        // Subtract a small amount to ensure consistent wrapping with the layout engine
        let inner_width = bounds.size.width - px(2.0);
        let line_height = font_size * 1.4;

        let mut y = px(0.0);
        let mut byte_offset = 0;

        for line in text.split_inclusive('\n') {
            let mut clean_line = line;
            let mut sep_len = 0;
            if clean_line.ends_with("\r\n") {
                clean_line = &clean_line[..clean_line.len()-2];
                sep_len = 2;
            } else if clean_line.ends_with('\n') {
                clean_line = &clean_line[..clean_line.len()-1];
                sep_len = 1;
            }

            let run = TextRun {
                len: clean_line.len(),
                font: font.clone(),
                color: Hsla::default(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            let shaped_lines = window.text_system().shape_text(
                SharedString::from(clean_line.to_string()),
                font_size,
                &[run],
                Some(inner_width),
                None,
            ).unwrap_or_default();

            let shaped_count = shaped_lines.len();
            for (i, shaped_line) in shaped_lines.into_iter().enumerate() {
                let line_len = shaped_line.len();
                let current_line_end = byte_offset + line_len;
                let is_last_shaped = i == shaped_count - 1;

                if index >= byte_offset && index < current_line_end {
                    let local_idx = index - byte_offset;
                    let x = shaped_line.unwrapped_layout.x_for_index(local_idx);
                    return point(bounds.left() + x, bounds.top() + y);
                }
                
                if index == current_line_end && is_last_shaped {
                    // If we are at the end of a logical line (before newline), we are on this line.
                    // Unless it's the very last line of text and sep_len == 0.
                    if index != text.len() || sep_len == 0 {
                         let x = shaped_line.unwrapped_layout.x_for_index(line_len);
                         return point(bounds.left() + x, bounds.top() + y);
                    }
                }

                y += line_height;
                byte_offset += line_len;
            }
            byte_offset += sep_len;
        }

        // If we reached here, we are probably at the end of text ending with newline
        point(bounds.left(), bounds.top() + y)
    }

    fn index_for_point(&self, point: Point<Pixels>, window: &Window) -> usize {
        let bounds = match self.input_bounds {
            Some(b) => b,
            None => return 0,
        };
        
        if self.commit_message.is_empty() {
            return 0;
        }

        let style = window.text_style();
        let font = style.font();
        let font_size = px(13.0);
        let text = &self.commit_message;
        // Subtract a small amount to ensure consistent wrapping with the layout engine
        let inner_width = bounds.size.width - px(2.0);
        let line_height = font_size * 1.4;

        let local_y = point.y - bounds.top();
        let local_x = point.x - bounds.left();
        
        let mut byte_offset = 0;
        let mut y = px(0.0);
        
        for line in text.split_inclusive('\n') {
            let mut clean_line = line;
            let mut sep_len = 0;
            if clean_line.ends_with("\r\n") {
                clean_line = &clean_line[..clean_line.len()-2];
                sep_len = 2;
            } else if clean_line.ends_with('\n') {
                clean_line = &clean_line[..clean_line.len()-1];
                sep_len = 1;
            }

            let run = TextRun {
                len: clean_line.len(),
                font: font.clone(),
                color: Hsla::default(),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            
            let shaped_lines = window.text_system().shape_text(
                SharedString::from(clean_line.to_string()),
                font_size,
                &[run],
                Some(inner_width),
                None,
            ).unwrap_or_default();

            for shaped_line in shaped_lines {
                let line_len = shaped_line.len();
                
                if local_y >= y && local_y < y + line_height {
                    let idx = shaped_line.unwrapped_layout.index_for_x(local_x).unwrap_or(line_len);
                    return byte_offset + idx;
                }
                
                y += line_height;
                byte_offset += line_len;
            }
            byte_offset += sep_len;
        }
        
        byte_offset
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
                            .flex()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_color(if self.mode == GitPanelMode::Changes { theme_text } else { theme_muted })
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .child("CHANGES")
                                    .on_mouse_down(MouseButton::Left, {
                                        let panel = panel.clone();
                                        move |_, _, cx| {
                                            panel.update(cx, |this, cx| {
                                                this.mode = GitPanelMode::Changes;
                                                cx.notify();
                                            });
                                        }
                                    })
                            )
                            .child(
                                div()
                                    .text_color(if self.mode == GitPanelMode::History { theme_text } else { theme_muted })
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .child("HISTORY")
                                    .on_mouse_down(MouseButton::Left, {
                                        let panel = panel.clone();
                                        move |_, _, cx| {
                                            panel.update(cx, |this, cx| {
                                                this.mode = GitPanelMode::History;
                                                cx.notify();
                                            });
                                        }
                                    })
                            )
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
            let msg = self.commit_message.clone();
            let is_empty = msg.is_empty();
            let display_text = if is_empty { "消息 (Ctrl+Enter 提交)".to_string() } else { msg };
            let text_color = if is_empty { theme_muted } else { theme_text };
            
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
                        .child(
                            div()
                                .relative()
                                .w_full()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .line_height(px(13.0 * 1.4))
                                        .text_color(Hsla::default().alpha(0.0))
                                        .whitespace_normal()
                                        .child(display_text.clone())
                                )
                                .child(
                                    canvas(
                                        |bounds, _window, _cx| bounds,
                                        {
                                            let focus = focus.clone();
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

                                                let style = window.text_style();
                                                let font_size = px(13.0);
                                                let font = style.font();
                                                
                                                // Shape text
                                                // Subtract a small amount to ensure consistent wrapping with the layout engine
                                                let inner_width = bounds.size.width - px(2.0);
                                                
                                                // Paint text and decorations
                                                let (cursor_idx, selection, marked_range) = panel.update(cx, |this, _| {
                                                    (this.commit_cursor, this.commit_selection.clone(), this.commit_message_marked_range.clone())
                                                });
                                                
                                                let is_focused = focus.is_focused(window);
                                                let line_height = font_size * 1.4;
                                                let mut line_y = bounds.top();
                                                let mut byte_offset = 0;

                                                for line in display_text.split_inclusive('\n') {
                                                    let mut clean_line = line;
                                                    let mut sep_len = 0;
                                                    if clean_line.ends_with("\r\n") {
                                                        clean_line = &clean_line[..clean_line.len()-2];
                                                        sep_len = 2;
                                                    } else if clean_line.ends_with('\n') {
                                                        clean_line = &clean_line[..clean_line.len()-1];
                                                        sep_len = 1;
                                                    }

                                                    let run = TextRun {
                                                        len: clean_line.len(),
                                                        font: font.clone(),
                                                        color: text_color.into(),
                                                        background_color: None,
                                                        underline: None,
                                                        strikethrough: None,
                                                    };

                                                    let shaped_lines = window.text_system().shape_text(
                                                        SharedString::from(clean_line.to_string()),
                                                        font_size,
                                                        &[run],
                                                        Some(inner_width),
                                                        None,
                                                    ).unwrap_or_default();
                                                    
                                                    let shaped_count = shaped_lines.len();
                                                    for (i, shaped_line) in shaped_lines.into_iter().enumerate() {
                                                        let line_len = shaped_line.len();
                                                        let current_line_end = byte_offset + line_len;
                                                        
                                                        // Draw Selection
                                                        if let Some(sel) = &selection {
                                                            let sel_start = sel.start.max(byte_offset);
                                                            let sel_end = sel.end.min(current_line_end);
                                                            
                                                            if sel_start < sel_end {
                                                                let x0 = shaped_line.unwrapped_layout.x_for_index(sel_start - byte_offset);
                                                                let x1 = shaped_line.unwrapped_layout.x_for_index(sel_end - byte_offset);
                                                                
                                                                window.paint_quad(fill(
                                                                    Bounds::from_corners(
                                                                        point(bounds.left() + x0, line_y),
                                                                        point(bounds.left() + x1, line_y + line_height),
                                                                    ),
                                                                    rgb(0x264f78),
                                                                ));
                                                            }
                                                        }

                                                        // Draw IME Marked Range
                                                        if let Some(marked) = &marked_range {
                                                             let start = utf16_index_to_byte(&display_text, marked.start);
                                                             let end = utf16_index_to_byte(&display_text, marked.end);
                                                             
                                                             let m_start = start.max(byte_offset);
                                                             let m_end = end.min(current_line_end);
                                                             
                                                             if m_start < m_end {
                                                                 let x0 = shaped_line.unwrapped_layout.x_for_index(m_start - byte_offset);
                                                                 let x1 = shaped_line.unwrapped_layout.x_for_index(m_end - byte_offset);
                                                                 
                                                                 window.paint_quad(fill(
                                                                     Bounds::new(
                                                                         point(bounds.left() + x0, line_y + line_height - px(1.0)),
                                                                         size(x1 - x0, px(1.0))
                                                                     ),
                                                                     theme_text,
                                                                 ));
                                                             }
                                                        }

                                                        // Draw Text
                                                        let _ = shaped_line.paint(point(bounds.left(), line_y), line_height, gpui::TextAlign::Left, None, window, cx);
                                                        
                                                        // Draw Cursor
                                                        if is_focused && !is_empty {
                                                             if cursor_idx >= byte_offset && cursor_idx < current_line_end {
                                                                 let local_idx = cursor_idx - byte_offset;
                                                                 let x = shaped_line.unwrapped_layout.x_for_index(local_idx);
                                                                 window.paint_quad(fill(
                                                                     Bounds::new(point(bounds.left() + x, line_y), size(px(1.5), line_height)),
                                                                     rgb(0xff007fd4),
                                                                 ));
                                                             } else if cursor_idx == current_line_end {
                                                                 let is_last_shaped = i == shaped_count - 1;
                                                                 if is_last_shaped {
                                                                      if cursor_idx != display_text.len() || sep_len == 0 {
                                                                           let x = shaped_line.unwrapped_layout.x_for_index(line_len);
                                                                           window.paint_quad(fill(
                                                                               Bounds::new(point(bounds.left() + x, line_y), size(px(1.5), line_height)),
                                                                               rgb(0xff007fd4),
                                                                           ));
                                                                      }
                                                                 }
                                                             }
                                                        } else if is_focused && is_empty && byte_offset == 0 {
                                                             window.paint_quad(fill(
                                                                 Bounds::new(point(bounds.left(), line_y), size(px(1.5), line_height)),
                                                                 rgb(0xff007fd4),
                                                             ));
                                                        }
                                                        
                                                        line_y += line_height;
                                                        byte_offset += line_len;
                                                    }
                                                    byte_offset += sep_len;
                                                }

                                                if is_focused && cursor_idx == byte_offset && byte_offset > 0 {
                                                    let ends_with_newline = display_text.ends_with('\n') || display_text.ends_with('\r');
                                                    if ends_with_newline {
                                                        window.paint_quad(fill(
                                                            Bounds::new(point(bounds.left(), line_y), size(px(1.5), line_height)),
                                                            rgb(0xff007fd4),
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    )
                                    .absolute()
                                    .inset_0()
                                    .size_full()
                                )
                        )
                        .on_mouse_down(MouseButton::Left, {

                            let focus = focus.clone();
                            let panel = panel.clone();
                            move |event, window, cx| {
                                focus.focus(window);
                                panel.update(cx, |this, cx| {
                                    let idx = this.index_for_point(event.position, window);
                                    this.commit_cursor = idx;
                                    this.commit_selection = None;
                                    this.drag_start_index = Some(idx);
                                    cx.notify();
                                });
                            }
                        })
                        .on_mouse_move({
                             let panel = panel.clone();
                             move |event, window, cx| {
                                 panel.update(cx, |this, cx| {
                                     if let Some(start) = this.drag_start_index {
                                         let idx = this.index_for_point(event.position, window);
                                         this.commit_cursor = idx;
                                         
                                         let s = start.min(idx);
                                         let e = start.max(idx);
                                         if s != e {
                                             this.commit_selection = Some(s..e);
                                         } else {
                                             this.commit_selection = None;
                                         }
                                         cx.notify();
                                     }
                                 });
                             }
                        })
                        .on_mouse_up(MouseButton::Left, {
                             let panel = panel.clone();
                             move |_, _, cx| {
                                 panel.update(cx, |this, _| {
                                     this.drag_start_index = None;
                                 });
                             }
                        })
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
                                .on_mouse_down(MouseButton::Left, {
                                    let panel = panel.clone();
                                    move |_, _w, cx| {
                                        panel.update(cx, |this, cx_inner| {
                                            this.commit_all();
                                            cx_inner.notify();
                                        });
                                    }
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

        let content = if self.mode == GitPanelMode::History {
            if let Some(selected_idx) = self.selected_commit_index {
                if selected_idx < self.commits.len() {
                    let commit = &self.commits[selected_idx];
                    let commit_changes = self.commit_changes.clone();
                    
                    div()
                        .flex_1()
                        .w_full()
                        .flex()
                        .flex_col()
                        .bg(theme_bg)
                        .child(
                            div()
                                .px(px(16.0))
                                .py(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .rounded_md()
                                        .hover(|s| s.bg(theme_hover))
                                        .child(
                                            div()
                                                .text_size(px(14.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(theme_text)
                                                .child("←")
                                        )
                                        .on_mouse_down(MouseButton::Left, {
                                            let panel = panel.clone();
                                            move |_, _, cx| {
                                                panel.update(cx, |this, cx| {
                                                    this.selected_commit_index = None;
                                                    cx.notify();
                                                });
                                            }
                                        })
                                )
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme_text)
                                        .child("Commit Details")
                                )
                        )
                        .child(
                            div()
                                .px(px(16.0))
                                .pb(px(12.0))
                                .border_b_1()
                                .border_color(rgb(0xff3c474d))
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme_text)
                                        .whitespace_normal()
                                        .child(commit.message.clone())
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                             div()
                                                .text_size(px(11.0))
                                                .text_color(theme_muted)
                                                .child(format!("{} • {}", commit.author, commit.short_id))
                                        )
                                )
                        )
                        .child(
                             list(
                                  self.commit_changes_list_state.clone(),
                                  move |index, _, _| {
                                      if index >= commit_changes.len() { return div().into_any_element(); }
                                      let ch = &commit_changes[index];
                                      let status_color = if ch.status.contains('M') {
                                          rgb(0xffe2c08d)
                                      } else if ch.status.contains('A') || ch.status.contains('?') {
                                          rgb(0xff73c991)
                                      } else if ch.status.contains('D') {
                                          rgb(0xfff14c4c)
                                      } else {
                                          theme_muted
                                      };
            
                                      div()
                                          .w_full()
                                          .h(px(24.0))
                                          .px(px(16.0))
                                          .flex()
                                          .items_center()
                                          .bg(theme_bg)
                                          .text_color(theme_text)
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
                                                  )
                                          )
                                          .into_any_element()
                                  }
                             )
                             .w_full()
                             .flex_1()
                        )
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            } else {
                let commits = self.commits.clone();
                div()
                    .flex_1()
                    .w_full()
                    .child(
                        list(
                            self.history_list_state.clone(),
                            move |index, _, _| {
                                if index >= commits.len() { return div().into_any_element(); }
                                let commit = &commits[index];
                                let short_id = commit.short_id.clone();
                                let message = commit.message.clone();
                                let author = commit.author.clone();
                                
                                div()
                                    .w_full()
                                    .py(px(6.0))
                                    .px(px(16.0))
                                    .border_b_1()
                                    .border_color(rgb(0xff3c474d))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme_hover))
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(theme_text)
                                                    .child(message)
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme_muted)
                                                    .child(short_id)
                                            )
                                    )
                                    .child(
                                        div()
                                            .mt(px(4.0))
                                            .flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(theme_muted)
                                                    .child(author)
                                            )
                                    )
                                    .on_mouse_down(MouseButton::Left, {
                                        let panel = panel.clone();
                                        move |_, _, cx| {
                                            panel.update(cx, |this, cx| {
                                                this.selected_commit_index = Some(index);
                                                this.load_commit_changes(index);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .into_any_element()
                            }
                        )
                        .w_full()
                        .h_full()
                    )
                    .into_any_element()
            }
        } else {
             div()
                .w_full()
                .h_full()
                .flex()
                .flex_col()
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
                .into_any_element()
        };

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme_bg)
            .child(header)
            .child(content)
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

        if range.start > range.end || range.end > self.commit_message.len() {
            return None;
        }

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
        let range = range_utf16
            .map(|r| utf16_range_to_byte_range(&self.commit_message, r))
            .or(self.commit_message_marked_range.clone())
            .or(self.commit_selection.clone())
            .unwrap_or(self.commit_cursor..self.commit_cursor);

        let start = range.start.min(self.commit_message.len());
        let end = range.end.min(self.commit_message.len());

        if start > end {
            return;
        }

        self.commit_message.replace_range(start..end, new_text);
        self.commit_cursor = start + new_text.len();
        self.commit_selection = None;
        self.commit_message_marked_range = None;
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
            .map(|r| utf16_range_to_byte_range(&self.commit_message, r))
            .or(self.commit_message_marked_range.clone())
            .or(self.commit_selection.clone())
            .unwrap_or(self.commit_cursor..self.commit_cursor);

        let start = range.start.min(self.commit_message.len());
        let end = range.end.min(self.commit_message.len());

        if start > end {
            return;
        }

        self.commit_message.replace_range(start..end, new_text);
        
        if !new_text.is_empty() {
            let marked_end = start + new_text.len();
            self.commit_message_marked_range = Some(start..marked_end);
        } else {
            self.commit_message_marked_range = None;
        }

        if let Some(new_range_utf16) = new_selected_range_utf16 {
            let new_range = utf16_range_to_byte_range(new_text, new_range_utf16);
            let sel_start = (start + new_range.start).min(self.commit_message.len());
            let sel_end = (start + new_range.end).min(self.commit_message.len());
            self.commit_selection = Some(sel_start..sel_end);
            self.commit_cursor = sel_end;
        } else {
            self.commit_selection = None;
            self.commit_cursor = (start + new_text.len()).min(self.commit_message.len());
        }
        cx.notify();
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
        let font = style.font();
        let font_size = px(13.0);
        let text = &self.commit_message;
        
        let run = TextRun {
            len: text.len(),
            font,
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        
        let inner_width = input_bounds.size.width;
        
        let lines = window.text_system().shape_text(
            SharedString::from(text.to_string()),
            font_size,
            &[run],
            Some(inner_width),
            None,
        );
        
        let start_byte = utf16_index_to_byte(text, range_utf16.start);
        let end_byte = utf16_index_to_byte(text, range_utf16.end);
        
        let mut byte_offset = 0;
        let mut y = px(0.0);
        let line_height = font_size * 1.4;
        
        for line in lines.into_iter().flatten() {
            let line_len = line.len();
            let line_end_offset = byte_offset + line_len;
            
            if start_byte >= byte_offset && start_byte <= line_end_offset {
                let local_start = start_byte - byte_offset;
                let local_end = (end_byte - byte_offset).min(line_len);
                
                let x0 = line.unwrapped_layout.x_for_index(local_start);
                let x1 = line.unwrapped_layout.x_for_index(local_end);
                
                return Some(Bounds::from_corners(
                    point(input_bounds.left() + x0, input_bounds.top() + y),
                    point(input_bounds.left() + x1, input_bounds.top() + y + line_height),
                ));
            }
            
            y += line_height;
            byte_offset += line_len;
        }
        
        Some(input_bounds)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(byte_index_to_utf16(&self.commit_message, self.index_for_point(point, window)))
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
