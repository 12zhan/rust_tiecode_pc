use super::tie_svg::tie_svg;
use gpui::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

#[derive(Clone)]
struct InlineNewItem {
    anchor_path: PathBuf,
    anchor_is_dir: bool,
    name: String,
    is_dir: bool,
    depth: usize,
    insert_index: usize,
    editing: bool,
}

#[derive(Clone)]
struct VirtualNode {
    name: String,
    is_dir: bool,
}

pub enum FileTreeEvent {
    OpenFile(PathBuf),
    ContextMenu {
        position: Point<Pixels>,
        path: PathBuf,
        is_dir: bool,
    },
    RequestMove {
        src: PathBuf,
        dst: PathBuf,
    },
    RequestDelete {
        path: PathBuf,
        is_dir: bool,
    },
}

pub struct FileTree {
    root_path: Option<PathBuf>,
    expanded_paths: HashSet<PathBuf>,
    visible_entries: Vec<FileEntry>,
    focus_handle: FocusHandle,
    list_state: ListState,
    fs_watch_active: bool,
    fs_watcher: Option<RecommendedWatcher>,
    fs_watcher_root: Option<PathBuf>,
    fs_event_rx: Option<mpsc::Receiver<()>>,
    drag_source: Option<PathBuf>,
    drag_hover: Option<PathBuf>,
    drag_active: bool,
    mouse_position: Point<Pixels>,
    drag_start_position: Option<Point<Pixels>>,
    selected_path: Option<PathBuf>,
    selection_time: Option<Instant>,
    animating: bool,
    pending_new_item: Option<InlineNewItem>,
    virtual_nodes: HashMap<PathBuf, Vec<VirtualNode>>,
}

impl EventEmitter<FileTreeEvent> for FileTree {}

impl FileTree {
    pub fn new(root_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let mut tree = Self {
            root_path: root_path.clone(),
            expanded_paths: HashSet::new(),
            visible_entries: Vec::new(),
            focus_handle: cx.focus_handle(),
            list_state: ListState::new(0, ListAlignment::Top, px(20.0)),
            fs_watch_active: false,
            fs_watcher: None,
            fs_watcher_root: None,
            fs_event_rx: None,
            drag_source: None,
            drag_hover: None,
            drag_active: false,
            mouse_position: Point::default(),
            drag_start_position: None,
            selected_path: None,
            selection_time: None,
            animating: false,
            pending_new_item: None,
            virtual_nodes: HashMap::new(),
        };
        tree.refresh_internal(false);
        tree.ensure_fs_watch(cx);
        tree
    }

    pub fn refresh(&mut self) {
        self.refresh_internal(true);
    }

    fn ensure_fs_watch(&mut self, cx: &mut Context<Self>) {
        if self.fs_watch_active {
            return;
        }
        self.fs_watch_active = true;
        self.sync_fs_watcher();
        cx.spawn(|entity: WeakEntity<FileTree>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(250))
                        .await;
                    let updated = entity.update(&mut cx, |this, cx| {
                        this.sync_fs_watcher();
                        if this.drain_fs_events() {
                            this.refresh_internal(true);
                            cx.notify();
                        }
                    });
                    if updated.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    fn sync_fs_watcher(&mut self) {
        let Some(root_path) = self.root_path.as_ref() else {
            self.fs_event_rx = None;
            self.fs_watcher = None;
            self.fs_watcher_root = None;
            return;
        };

        if self.fs_watcher_root.as_ref() == Some(root_path) && self.fs_watcher.is_some() {
            return;
        }

        self.fs_event_rx = None;
        self.fs_watcher = None;
        self.fs_watcher_root = None;

        let (tx, rx) = mpsc::channel::<()>();
        let mut watcher = match notify::recommended_watcher(move |_res| {
            let _ = tx.send(());
        }) {
            Ok(watcher) => watcher,
            Err(err) => {
                println!("FileTree fs watcher init failed: {:?}", err);
                return;
            }
        };

        if let Err(err) = watcher.watch(root_path, RecursiveMode::Recursive) {
            println!("FileTree fs watcher watch failed: {:?} ({:?})", err, root_path);
            return;
        }

        self.fs_watcher = Some(watcher);
        self.fs_watcher_root = Some(root_path.clone());
        self.fs_event_rx = Some(rx);
    }

    fn drain_fs_events(&mut self) -> bool {
        let mut changed = false;
        let mut disconnected = false;

        if let Some(rx) = self.fs_event_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(()) => changed = true,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        if disconnected {
            self.fs_event_rx = None;
            self.fs_watcher = None;
            self.fs_watcher_root = None;
        }

        changed
    }

    fn refresh_internal(&mut self, preserve_scroll: bool) {
        let scroll_top = if preserve_scroll {
            Some(self.list_state.logical_scroll_top())
        } else {
            None
        };

        self.visible_entries.clear();
        let root_path = match self.root_path.as_ref() {
            Some(path) => path.clone(),
            None => {
                self.list_state.reset(0);
                return;
            }
        };
        let root_name = root_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| root_path.to_string_lossy().to_string());
        let root_expanded = self.expanded_paths.contains(&root_path);
        self.visible_entries.push(FileEntry {
            path: root_path.clone(),
            name: root_name,
            is_dir: true,
            depth: 0,
            is_expanded: root_expanded,
        });
        if root_expanded {
            self.append_entries(&root_path.clone(), 1);
        }
        if self.pending_new_item.is_some() {
            let (insert_index, depth) = {
                let pending = self.pending_new_item.as_ref().expect("checked above");
                self.inline_insert_position(&pending.anchor_path, pending.anchor_is_dir)
            };
            if let Some(pending) = self.pending_new_item.as_mut() {
                pending.insert_index = insert_index;
                pending.depth = depth;
            }
            self.list_state.reset(self.visible_entries.len() + 1);
        } else {
            self.list_state.reset(self.visible_entries.len());
        }

        if let Some(scroll_top) = scroll_top {
            self.list_state.scroll_to(scroll_top);
        }
    }

    pub fn set_root_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.root_path = Some(path);
        self.expanded_paths.clear();
        self.refresh_internal(false);
        self.sync_fs_watcher();
        cx.notify();
    }

    pub fn toggle_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.toggle_expand(path, cx);
    }

    pub fn begin_inline_create(
        &mut self,
        anchor_path: PathBuf,
        anchor_is_dir: bool,
        create_dir: bool,
        cx: &mut Context<Self>,
    ) {
        self.remove_inline_item();
        if anchor_is_dir && !self.expanded_paths.contains(&anchor_path) {
            self.expanded_paths.insert(anchor_path.clone());
            self.refresh_internal(true);
        }

        let (insert_index, depth) = self.inline_insert_position(&anchor_path, anchor_is_dir);
        self.pending_new_item = Some(InlineNewItem {
            anchor_path,
            anchor_is_dir,
            name: String::new(),
            is_dir: create_dir,
            depth,
            insert_index,
            editing: true,
        });
        self.list_state.splice(insert_index..insert_index, 1);
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window) {
        self.focus_handle.focus(window);
    }

    fn inline_insert_position(&self, anchor_path: &Path, anchor_is_dir: bool) -> (usize, usize) {
        if let Some((index, entry)) = self
            .visible_entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.path == anchor_path)
        {
            let depth = if anchor_is_dir {
                entry.depth + 1
            } else {
                entry.depth
            };
            return (index + 1, depth);
        }
        (self.visible_entries.len(), 0)
    }

    fn remove_inline_item(&mut self) {
        if let Some(pending) = self.pending_new_item.take() {
            self.list_state
                .splice(pending.insert_index..pending.insert_index + 1, 0);
        }
    }

    fn cancel_inline_item(&mut self, cx: &mut Context<Self>) {
        self.remove_inline_item();
        cx.notify();
    }

    fn commit_inline_item(&mut self, cx: &mut Context<Self>) {
        let pending = match self.pending_new_item.take() {
            Some(pending) => pending,
            None => return,
        };

        self.list_state
            .splice(pending.insert_index..pending.insert_index + 1, 0);

        let name = pending.name.trim().to_string();
        if name.is_empty() {
            cx.notify();
            return;
        }

        let root_path = match self.root_path.as_ref() {
            Some(path) => path,
            None => {
                cx.notify();
                return;
            }
        };
        let parent = if pending.anchor_is_dir {
            pending.anchor_path
        } else {
            pending
                .anchor_path
                .parent()
                .unwrap_or(root_path)
                .to_path_buf()
        };

        let segments: Vec<String> = name
            .split(|c| c == '/' || c == '\\')
            .filter(|part| !part.is_empty())
            .map(|part| part.to_string())
            .collect();
        if segments.is_empty() {
            cx.notify();
            return;
        }

        let mut current_parent = parent.clone();
        if pending.is_dir {
            for segment in segments {
                let child_path = current_parent.join(&segment);
                self.add_virtual_node(current_parent.clone(), segment, true);
                self.expanded_paths.insert(current_parent.clone());
                current_parent = child_path;
            }
            self.expanded_paths.insert(current_parent.clone());
            self.selected_path = Some(current_parent);
        } else {
            let (dir_parts, file_part) = segments.split_at(segments.len() - 1);
            for segment in dir_parts {
                let child_path = current_parent.join(segment);
                self.add_virtual_node(current_parent.clone(), segment.clone(), true);
                self.expanded_paths.insert(current_parent.clone());
                current_parent = child_path;
            }
            let file_name = file_part[0].clone();
            self.add_virtual_node(current_parent.clone(), file_name.clone(), false);
            self.expanded_paths.insert(current_parent.clone());
            self.selected_path = Some(current_parent.join(file_name));
        }

        self.selection_time = Some(Instant::now());
        self.refresh_internal(true);
        cx.notify();
    }

    fn backspace_inline_item(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = self.pending_new_item.as_mut() {
            if !pending.editing || pending.name.is_empty() {
                return;
            }
            let prev = prev_char_boundary(&pending.name, pending.name.len());
            pending.name.truncate(prev);
            cx.notify();
        }
    }

    fn append_inline_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if text.is_empty() {
            return;
        }
        if let Some(pending) = self.pending_new_item.as_mut() {
            if !pending.editing {
                return;
            }
            pending.name.push_str(text);
            cx.notify();
        }
    }

    fn append_entries(&mut self, path: &Path, depth: usize) {
        let mut children: Vec<(PathBuf, String, bool)> = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let child_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                children.push((child_path, name, is_dir));
            }
        }

        if let Some(virtuals) = self.virtual_nodes.get(path) {
            for virtual_node in virtuals {
                let child_path = path.join(&virtual_node.name);
                children.push((child_path, virtual_node.name.clone(), virtual_node.is_dir));
            }
        }

        // Sort: Directories first, then files
        children.sort_by(|a, b| {
            if a.2 == b.2 {
                a.1.cmp(&b.1)
            } else {
                b.2.cmp(&a.2)
            }
        });

        for (child_path, name, is_dir) in children {
            let is_expanded = self.expanded_paths.contains(&child_path);

            self.visible_entries.push(FileEntry {
                path: child_path.clone(),
                name,
                is_dir,
                depth,
                is_expanded,
            });

            if is_dir && is_expanded {
                self.append_entries(&child_path, depth + 1);
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
        self.refresh_internal(true);
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

    fn add_virtual_node(&mut self, parent: PathBuf, name: String, is_dir: bool) {
        let entry = self.virtual_nodes.entry(parent).or_insert_with(Vec::new);
        if entry
            .iter()
            .any(|node| node.name == name && node.is_dir == is_dir)
        {
            return;
        }
        entry.push(VirtualNode { name, is_dir });
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let editing = self
            .pending_new_item
            .as_ref()
            .map(|item| item.editing)
            .unwrap_or(false);
        if !editing {
            return;
        }

        let key = event.keystroke.key.as_str();
        match key {
            "enter" => {
                self.commit_inline_item(cx);
                cx.stop_propagation();
                return;
            }
            "escape" => {
                self.cancel_inline_item(cx);
                cx.stop_propagation();
                return;
            }
            "backspace" | "delete" => {
                self.backspace_inline_item(cx);
                cx.stop_propagation();
                return;
            }
            _ => {}
        }

        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return;
        }

        if let Some(text) = event.keystroke.key_char.as_ref() {
            if !text.chars().all(|c| c.is_control()) {
                self.append_inline_text(text, cx);
                cx.stop_propagation();
            }
        } else if key == "space" {
            self.append_inline_text(" ", cx);
            cx.stop_propagation();
        }
    }
}

impl Render for FileTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_surface = rgb(0xff252526);
        let drop_highlight = rgb(0xff3c474d);
        let visible_entries = self.visible_entries.clone();
        let drag_hover = self.drag_hover.clone();
        let pending_new_item = self.pending_new_item.clone();
        let view = cx.entity().clone();

        let view_mousemove = view.clone();

        let root_path = match self.root_path.clone() {
            Some(path) => path,
            None => {
                return div()
                    .w_full()
                    .h_full()
                    .bg(theme_surface)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(rgb(0xff8b949e))
                    .child("拖拽文件夹打开");
            }
        };
        let selected_path = self.selected_path.clone();
        let selection_time = self.selection_time;
        let selected_parent_path = selected_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|path| path.to_path_buf());
        let selected_parent_depth = selected_parent_path
            .as_ref()
            .and_then(|path| path.strip_prefix(&root_path).ok())
            .map(|relative| relative.components().count());
        let pending_index = pending_new_item.as_ref().map(|item| item.insert_index);
        let selected_row_index = selected_path
            .as_ref()
            .and_then(|path| visible_entries.iter().position(|e| &e.path == path))
            .map(|index| {
                if let Some(pending_index) = pending_index {
                    if index >= pending_index {
                        index + 1
                    } else {
                        index
                    }
                } else {
                    index
                }
            });

        div()
            .w_full()
            .h_full()
            .bg(theme_surface)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
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
                    let total_len =
                        visible_entries.len() + pending_new_item.as_ref().map(|_| 1).unwrap_or(0);
                    if ix >= total_len {
                        return div().id("empty").into_any_element();
                    }

                    if let Some(pending) = pending_new_item.as_ref() {
                        if ix == pending.insert_index {
                            let theme_text = rgb(0xffcccccc);
                            let placeholder_color = rgb(0xff8b949e);
                            let theme_selected = rgb(0xff37373d);
                            let placeholder = if pending.is_dir {
                                "新建文件夹"
                            } else {
                                "新建文件"
                            };
                            let display_text = if pending.name.is_empty() {
                                placeholder.to_string()
                            } else {
                                pending.name.clone()
                            };
                            let text_color = if pending.name.is_empty() {
                                placeholder_color
                            } else {
                                theme_text
                            };
                            let icon = if pending.is_dir {
                                folder_icon().into_any_element()
                            } else {
                                file_icon(&pending.name).into_any_element()
                            };

                            let mut row = div()
                                .id(SharedString::from(format!(
                                    "inline-new-{}",
                                    pending.insert_index
                                )))
                                .relative()
                                .flex()
                                .items_center()
                                .h(px(24.0))
                                .pl(px(10.0 + pending.depth as f32 * 10.0))
                                .bg(if pending.editing {
                                    theme_selected
                                } else {
                                    theme_surface
                                });

                            for i in 0..pending.depth {
                                row = row.child(
                                    div()
                                        .absolute()
                                        .left(px(10.0 + i as f32 * 10.0))
                                        .top(px(0.0))
                                        .w(px(1.0))
                                        .h_full()
                                        .bg(rgb(0x303030)),
                                );
                            }

                            row = row.child(div().mr(px(6.0)).child(icon)).child(
                                div()
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.0))
                                    .text_color(text_color)
                                    .child(display_text)
                                    .child(if pending.editing {
                                        div()
                                            .ml(px(2.0))
                                            .w(px(1.0))
                                            .h(px(14.0))
                                            .bg(theme_text)
                                            .into_any_element()
                                    } else {
                                        div().into_any_element()
                                    }),
                            );

                            return row.into_any_element();
                        }
                    }

                    let entry_index = if let Some(pending_index) = pending_index {
                        if ix > pending_index {
                            ix - 1
                        } else {
                            ix
                        }
                    } else {
                        ix
                    };

                    let entry = &visible_entries[entry_index];
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
                    let view_right = view.clone();
                    let path_click = path_clone.clone();
                    let path_down = path_clone.clone();
                    let path_move = path_clone.clone();
                    let path_right = path_clone.clone();

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
                    for i in 0..depth {
                        let mut is_active = false;
                        if let (Some(parent_path), Some(parent_depth)) =
                            (&selected_parent_path, selected_parent_depth)
                        {
                            if i == parent_depth && path.starts_with(parent_path) {
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
                        .on_mouse_down(MouseButton::Right, move |e, _window, cx| {
                            view_right.update(cx, |this, cx| {
                                println!("FileTree right-click position: {:?}", e.position);
                                this.selected_path = Some(path_right.clone());
                                this.selection_time = Some(Instant::now());
                                this.ensure_animation(cx);
                                cx.emit(FileTreeEvent::ContextMenu {
                                    position: e.position,
                                    path: path_right.clone(),
                                    is_dir,
                                });
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
                                            } else if let Some(parent) = path_move.parent() {
                                                parent.to_path_buf()
                                            } else if let Some(root_path) = this.root_path.as_ref() {
                                                root_path.clone()
                                            } else {
                                                return;
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
                                    if let (Some(src), Some(dst_dir)) = (src, dst) {
                                        if dst_dir != src && !this.is_descendant(&src, &dst_dir) {
                                            if let Some(name) = src.file_name() {
                                                let dst = dst_dir.join(name);
                                                if dst != src {
                                                    cx.emit(FileTreeEvent::RequestMove { src, dst });
                                                }
                                            }
                                        } else {
                                            println!("Invalid move: {:?} -> {:?}", src, dst_dir);
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
        .original_colors(true)
}

fn folder_open_icon() -> impl IntoElement {
    tie_svg()
        .path("assets/icons/folder_dark.svg") // No open variant in list
        .size(px(16.0))
        .original_colors(true)
}

pub fn file_icon(name: &str) -> impl IntoElement {
    tie_svg()
        .path(get_icon_path(name))
        .size(px(16.0))
        .original_colors(true)
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
