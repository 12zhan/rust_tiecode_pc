use super::tie_svg::tie_svg;
use gpui::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

pub enum FileTreeEvent {
    OpenFile(PathBuf),
}

pub struct FileTree {
    root_path: PathBuf,
    expanded_paths: HashSet<PathBuf>,
    visible_entries: Vec<FileEntry>,
    focus_handle: FocusHandle,
    list_state: ListState,
    drag_source: Option<PathBuf>,
    drag_hover: Option<PathBuf>,
    drag_active: bool,
    mouse_position: Point<Pixels>,
    drag_start_position: Option<Point<Pixels>>,
    selected_path: Option<PathBuf>,
    selection_time: Option<Instant>,
    animating: bool,
}

impl EventEmitter<FileTreeEvent> for FileTree {}

impl FileTree {
    pub fn new(root_path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut tree = Self {
            root_path: root_path.clone(),
            expanded_paths: HashSet::new(),
            visible_entries: Vec::new(),
            focus_handle: cx.focus_handle(),
            list_state: ListState::new(0, ListAlignment::Top, px(20.0)),
            drag_source: None,
            drag_hover: None,
            drag_active: false,
            mouse_position: Point::default(),
            drag_start_position: None,
            selected_path: None,
            selection_time: None,
            animating: false,
        };
        tree.refresh();
        tree
    }

    pub fn refresh(&mut self) {
        self.visible_entries.clear();
        self.append_entries(&self.root_path.clone(), 0);
        self.list_state.reset(self.visible_entries.len());
    }

    pub fn set_root_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.root_path = path;
        self.expanded_paths.clear();
        self.refresh();
        cx.notify();
    }

    fn append_entries(&mut self, path: &Path, depth: usize) {
        if let Ok(entries) = fs::read_dir(path) {
            let mut entries_vec: Vec<_> = entries.filter_map(|e| e.ok()).collect();

            // Sort: Directories first, then files
            entries_vec.sort_by(|a, b| {
                let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if a_is_dir == b_is_dir {
                    a.file_name().cmp(&b.file_name())
                } else {
                    b_is_dir.cmp(&a_is_dir)
                }
            });

            for entry in entries_vec {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let is_expanded = self.expanded_paths.contains(&path);

                self.visible_entries.push(FileEntry {
                    path: path.clone(),
                    name,
                    is_dir,
                    depth,
                    is_expanded,
                });

                if is_dir && is_expanded {
                    self.append_entries(&path, depth + 1);
                }
            }
        }
    }

    fn ensure_animation(&mut self, cx: &mut Context<Self>) {
        if self.animating {
            return;
        }
        self.animating = true;
        cx.spawn(|entity: WeakEntity<FileTree>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(16))
                        .await;
                    let keep_running = entity
                        .update(&mut cx, |this, cx| this.tick_animation(cx))
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
        if let Some(time) = self.selection_time {
            let elapsed = time.elapsed().as_secs_f32();
            let max_dist = self.visible_entries.len() as f32;
            let speed = 60.0; // items per second
            let current_dist = elapsed * speed;

            cx.notify();

            if current_dist > max_dist + 10.0 {
                self.animating = false;
                return false;
            }
            return true;
        }
        self.animating = false;
        false
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_active
    }

    pub fn drag_source(&self) -> Option<PathBuf> {
        self.drag_source.clone()
    }

    pub fn mouse_position(&self) -> Point<Pixels> {
        self.mouse_position
    }

    fn toggle_expand(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.expanded_paths.contains(&path) {
            self.expanded_paths.remove(&path);
        } else {
            self.expanded_paths.insert(path);
        }
        self.refresh();
        cx.notify();
    }

    fn open_file(&self, path: &Path, cx: &mut Context<Self>) {
        cx.emit(FileTreeEvent::OpenFile(path.to_path_buf()));
    }

    fn is_descendant(&self, src: &Path, dst: &Path) -> bool {
        let mut cur = Some(dst);
        while let Some(p) = cur {
            if p == src {
                return true;
            }
            cur = p.parent();
        }
        false
    }
}

impl Render for FileTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_surface = rgb(0xff252526);
        let drop_highlight = rgb(0xff3c474d);
        let visible_entries = self.visible_entries.clone();
        let drag_hover = self.drag_hover.clone();
        let view = cx.entity().clone();

        let view_mousemove = view.clone();

        let root_path = self.root_path.clone();
        let selected_path = self.selected_path.clone();
        let selection_time = self.selection_time;
        let selected_row_index = selected_path
            .as_ref()
            .and_then(|path| visible_entries.iter().position(|e| &e.path == path));

        div()
            .w_full()
            .h_full()
            .bg(theme_surface)
            .track_focus(&self.focus_handle)
            .on_mouse_move(move |e: &MouseMoveEvent, _window, cx| {
                view_mousemove.update(cx, |this, cx| {
                    this.mouse_position = e.position;
                    if this.drag_active {
                        cx.notify();
                    }
                });
            })
            .child(
                list(self.list_state.clone(), move |ix, _window, _cx| {
                    if ix >= visible_entries.len() {
                        return div().id("empty").into_any_element();
                    }

                    let entry = &visible_entries[ix];
                    let path = entry.path.clone();
                    let is_dir = entry.is_dir;
                    let depth = entry.depth;
                    let is_expanded = entry.is_expanded;
                    let name = entry.name.clone();

                    let view = view.clone();
                    let path_clone = path.clone();

                    let theme_hover = rgb(0xff2a2d2e);
                    let theme_text = rgb(0xffcccccc);
                    let theme_selected = rgb(0xff37373d);

                    let is_drop_target =
                        drag_hover.as_ref().map(|p| p == &path).unwrap_or(false) && is_dir;
                    let is_selected = selected_path.as_ref().map(|p| p == &path).unwrap_or(false);

                    let view_click = view.clone();
                    let view_down = view.clone();
                    let view_move = view.clone();
                    let view_up = view.clone();
                    let path_click = path_clone.clone();
                    let path_down = path_clone.clone();
                    let path_move = path_clone.clone();

                    let mut row = div()
                        .id(SharedString::from(path.to_string_lossy().to_string()))
                        .relative()
                        .flex()
                        .items_center()
                        .h(px(24.0))
                        .pl(px(10.0 + depth as f32 * 10.0))
                        .hover(|s| s.bg(theme_hover))
                        .cursor_pointer();

                    if is_selected {
                        row = row.bg(theme_selected);
                    }

                    if is_drop_target {
                        row = row.bg(drop_highlight);
                    }

                    // Indentation Guides
                    let relative_path = path.strip_prefix(&root_path).ok();
                    let relative_selected = selected_path
                        .as_ref()
                        .and_then(|p| p.strip_prefix(&root_path).ok());

                    for i in 0..depth {
                        let mut is_active = false;
                        if let (Some(rp), Some(rsp)) = (relative_path, relative_selected) {
                            let rp_comps: Vec<_> = rp.components().take(i + 1).collect();
                            let rsp_comps: Vec<_> = rsp.components().take(i + 1).collect();
                            if rp_comps == rsp_comps {
                                is_active = true;
                            }
                        }

                        // Animation logic
                        if is_active {
                            if let (Some(sel_idx), Some(time)) =
                                (selected_row_index, selection_time)
                            {
                                let elapsed = time.elapsed().as_secs_f32();
                                let speed = 60.0; // items per second
                                let max_dist = elapsed * speed;
                                let dist = (ix as isize - sel_idx as isize).abs() as f32;
                                if dist > max_dist {
                                    is_active = false;
                                }
                            }
                        }

                        row = row.child(
                            div()
                                .absolute()
                                .left(px(10.0 + i as f32 * 10.0))
                                .top(px(0.0))
                                .w(px(1.0))
                                .h_full()
                                .bg(if is_active {
                                    rgb(0x505050)
                                } else {
                                    rgb(0x303030)
                                }),
                        );
                    }

                    row = row
                        .on_click(move |_, _window, cx| {
                            view_click.update(cx, |this, cx| {
                                this.selected_path = Some(path_click.clone());
                                this.selection_time = Some(Instant::now());
                                this.ensure_animation(cx);
                                if is_dir {
                                    this.toggle_expand(path_click.clone(), cx);
                                } else {
                                    this.open_file(&path_click, cx);
                                }
                                cx.notify();
                            });
                        })
                        .on_mouse_down(MouseButton::Left, move |e, _window, cx| {
                            view_down.update(cx, |this, cx| {
                                this.drag_source = Some(path_down.clone());
                                this.drag_start_position = Some(e.position);
                                this.drag_active = false;
                                this.drag_hover = None;
                                cx.notify();
                            });
                        })
                        .on_mouse_move(move |e, _window, cx| {
                            view_move.update(cx, |this, cx| {
                                if let Some(start) = this.drag_start_position {
                                    let diff = e.position - start;
                                    let dist_sq =
                                        f32::from(diff.x).powi(2) + f32::from(diff.y).powi(2);
                                    if dist_sq > 25.0 {
                                        if this.drag_source.is_some() {
                                            this.drag_active = true;
                                            let hover_target = if is_dir {
                                                path_move.clone()
                                            } else {
                                                path_move
                                                    .parent()
                                                    .unwrap_or(&this.root_path)
                                                    .to_path_buf()
                                            };
                                            this.drag_hover = Some(hover_target);
                                            cx.notify();
                                        }
                                    }
                                }
                            });
                        })
                        .on_mouse_up(MouseButton::Left, move |_, _window, cx| {
                            view_up.update(cx, |this, cx| {
                                this.drag_start_position = None;
                                if this.drag_active {
                                    let src = this.drag_source.take();
                                    let dst = this.drag_hover.take();
                                    this.drag_active = false;
                                    if let (Some(src), Some(dst)) = (src, dst) {
                                        if dst != src && !this.is_descendant(&src, &dst) {
                                            // UI-only mode: Do not move files
                                            println!("UI Drag: Would move {:?} -> {:?}", src, dst);
                                            cx.notify();
                                        } else {
                                            println!("Invalid move: {:?} -> {:?}", src, dst);
                                        }
                                    }
                                } else {
                                    this.drag_source = None;
                                }
                            });
                        })
                        .child(div().mr(px(6.0)).child(if is_dir {
                            if is_expanded {
                                folder_open_icon().into_any_element()
                            } else {
                                folder_icon().into_any_element()
                            }
                        } else {
                            file_icon(&name).into_any_element()
                        }))
                        .child(div().text_size(px(13.0)).text_color(theme_text).child(name));

                    row.into_any_element()
                })
                .w_full()
                .h_full(),
            )
    }
}

fn get_icon_path(name: &str) -> String {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        "cargo.toml" => return "assets/icons/cargo_dark.svg".to_string(),
        "cargo.lock" => return "assets/icons/cargoLock_dark.svg".to_string(),
        ".gitignore" => return "assets/icons/gitignore.svg".to_string(),
        "cmakelists.txt" => return "assets/icons/CMake_dark.svg".to_string(),
        _ => {}
    }

    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let icon = match ext.as_str() {
        "t" => "tie_file.svg",
        "rs" => "rustFile_dark.svg",
        "toml" => "toml_dark.svg",
        "json" => "json_dark.svg",
        "md" => "markdown_dark.svg",
        "js" => "javaScript_dark.svg",
        "ts" => "typeScript_dark.svg",
        "tsx" => "tsx_dark.svg",
        "jsx" => "jsx_dark.svg",
        "html" => "html_dark.svg",
        "css" => "css.svg",
        "svg" | "png" | "jpg" | "jpeg" | "ico" => "image_dark.svg",
        "lock" => "lock_dark.svg",
        "yml" | "yaml" => "yaml_dark.svg",
        "xml" => "xml_dark.svg",
        "sql" => "sql_dark.svg",
        "sh" => "shell_dark.svg",
        "c" => "c_dark.svg",
        "cpp" | "cc" | "cxx" => "cpp_dark.svg",
        "h" => "h_dark.svg",
        "py" => "python.svg",
        "java" => "java_dark.svg",
        "lua" => "lua.svg",
        "go" => "go_dark.svg",
        "php" => "php_dark.svg",
        "rb" => "ruby_dark.svg",
        "zip" | "tar" | "gz" | "7z" | "rar" => "archive_dark.svg",
        "txt" => "text_dark.svg",
        _ => "anyType_dark.svg",
    };
    format!("assets/icons/{}", icon)
}

fn folder_icon() -> impl IntoElement {
    tie_svg()
        .path("assets/icons/folder_dark.svg")
        .size(px(16.0))
        .text_color(rgb(0x90a4ae)) // Material Icon Theme default folder color
}

fn folder_open_icon() -> impl IntoElement {
    tie_svg()
        .path("assets/icons/folder_dark.svg") // No open variant in list
        .size(px(16.0))
        .text_color(rgb(0x90a4ae))
}

pub fn file_icon(name: &str) -> impl IntoElement {
    tie_svg()
        .path(get_icon_path(name))
        .size(px(16.0))
        .original_colors(true)
}
