use gpui::*;
use image::GenericImageView;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ImageViewer {
    path: Option<PathBuf>,
    scale: f32,
    offset: Point<Pixels>,
    dragging: bool,
    drag_start: Option<Point<Pixels>>,
    image_size: Option<(u32, u32)>,
    focus_handle: FocusHandle,
}

impl ImageViewer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            path: None,
            scale: 1.0,
            offset: point(px(0.0), px(0.0)),
            dragging: false,
            drag_start: None,
            image_size: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn open_image(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.scale = 1.0;
        self.offset = point(px(0.0), px(0.0));
        self.dragging = false;
        self.drag_start = None;
        // Downscale large images to avoid huge GPU textures causing stutter
        let final_path = match image::open(&path) {
            Ok(img) => {
                let (w, h) = img.dimensions();
                self.image_size = Some((w, h));
                if w > 1920 || h > 1080 {
                    let resized = img.thumbnail(1920, 1080);
                    let (rw, rh) = resized.dimensions();
                    self.image_size = Some((rw, rh));
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos();
                    let temp_path = std::env::temp_dir()
                        .join(format!("tiecode_image_cache_{}.jpg", timestamp));
                    if resized.save(&temp_path).is_ok() {
                        temp_path
                    } else {
                        path.clone()
                    }
                } else {
                    path.clone()
                }
            }
            Err(_) => path.clone(),
        };
        self.path = Some(final_path);
        cx.notify();
    }
}

impl Render for ImageViewer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let entity = _cx.entity();
        let view_for_down = entity.clone();
        let view_for_up = entity.clone();
        let view_for_move = entity.clone();
        let view_for_wheel = entity.clone();
        let view = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0xff2d353b))
            .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                view_for_down.update(cx, |this, cx_inner| {
                    (*this).dragging = true;
                    (*this).drag_start = Some(event.position);
                    cx_inner.notify();
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _window, cx| {
                view_for_up.update(cx, |this, cx_inner| {
                    (*this).dragging = false;
                    (*this).drag_start = None;
                    cx_inner.notify();
                });
            })
            .on_mouse_move(move |event, _window, cx| {
                view_for_move.update(cx, |this, cx_inner| {
                    if (*this).dragging {
                        if let Some(start) = (*this).drag_start {
                            let dx = event.position.x - start.x;
                            let dy = event.position.y - start.y;
                            (*this).offset.x += dx;
                            (*this).offset.y += dy;
                            (*this).drag_start = Some(event.position);
                        }
                    }
                    cx_inner.notify();
                });
            })
            .on_scroll_wheel(move |event, _window, cx| {
                view_for_wheel.update(cx, |this, cx_inner| {
                    let delta = event.delta.pixel_delta(px(0.0)).y;
                    if delta != px(0.0) {
                        let old_scale = (*this).scale;
                        let factor = if delta > px(0.0) { 1.1 } else { 0.9 };
                        let new_scale = (old_scale * factor).clamp(0.1, 8.0);
                        let pos = event.position;
                        let ox = (*this).offset.x;
                        let oy = (*this).offset.y;
                        let nx = pos.x - (pos.x - ox) * (new_scale / old_scale);
                        let ny = pos.y - (pos.y - oy) * (new_scale / old_scale);
                        (*this).offset = point(nx, ny);
                        (*this).scale = new_scale;
                    }
                    cx_inner.notify();
                });
            });

        if let Some(path) = self.path.clone() {
            let (w, h) = self.image_size.unwrap_or((800, 600));
            let sw = px(w as f32 * self.scale);
            let sh = px(h as f32 * self.scale);
            view.child(
                img(path)
                    .absolute()
                    .left(self.offset.x)
                    .top(self.offset.y)
                    .w(sw)
                    .h(sh),
            )
        } else {
            view.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(rgb(0xffa9b1b6))
                    .child("打开图片以预览"),
            )
        }
    }
}
