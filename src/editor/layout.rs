use gpui::*;

#[derive(Clone, Copy)]
pub struct EditorLayout {
    pub font_size: Pixels,
    pub scroll_offset: Point<Pixels>,
    pub last_bounds: Option<Bounds<Pixels>>,
}

impl EditorLayout {
    pub fn new() -> Self {
        Self {
            font_size: px(14.0),
            scroll_offset: point(px(0.0), px(0.0)),
            last_bounds: None,
        }
    }

    pub fn line_height(&self) -> Pixels {
        self.font_size * 1.4
    }

    pub fn gutter_width(&self, max_digits: usize) -> Pixels {
        let digit_width = self.font_size * 0.75; // Approximation for digit width
        let padding = px(16.0); // 8px left + 8px right
        digit_width * (max_digits as f32) + padding
    }

    pub fn text_x(&self, bounds: Bounds<Pixels>, max_digits: usize) -> Pixels {
        bounds.left() + self.gutter_width(max_digits) + px(8.0) + self.scroll_offset.x
    }

    pub fn line_y(&self, bounds: Bounds<Pixels>, line_index: usize) -> Pixels {
        bounds.top() + self.line_height() * line_index as f32 + self.scroll_offset.y
    }

    pub fn line_index_for_y(&self, bounds: Bounds<Pixels>, y: Pixels) -> usize {
        let local_y = y - bounds.top() - self.scroll_offset.y;
        let line_height = self.line_height();
        if line_height <= px(0.0) {
            return 0;
        }
        (local_y / line_height).floor().max(0.0) as usize
    }

    pub fn scroll(&mut self, delta: Point<Pixels>, max_scroll: Point<Pixels>) {
        self.scroll_offset = self.scroll_offset + delta;
        self.scroll_offset.y = self.scroll_offset.y.clamp(-max_scroll.y, px(0.0));
        self.scroll_offset.x = self.scroll_offset.x.clamp(-max_scroll.x, px(0.0));
    }

    pub fn zoom(&mut self, delta: Pixels) {
        self.font_size = (self.font_size + delta).clamp(px(6.0), px(100.0));
    }

    // Scrollbar helpers
    pub fn scrollbar_width(&self) -> Pixels {
        px(14.0)
    }

    pub fn content_height(&self, line_count: usize) -> Pixels {
        self.line_height() * line_count as f32
    }

    pub fn thumb_bounds(
        &self,
        bounds: Bounds<Pixels>,
        content_height: Pixels,
    ) -> Bounds<Pixels> {
        let view_height = bounds.size.height;
        let scrollbar_width = self.scrollbar_width();
        
        if content_height <= view_height {
            return Bounds::default();
        }

        let total_scrollable_height = content_height + view_height * 0.5; // Allow some overscroll/padding
        
        let thumb_height = (view_height / total_scrollable_height * view_height).max(px(20.0));
        let max_thumb_y = view_height - thumb_height;
        
        // Calculate thumb position based on scroll offset
        // scroll_offset.y is negative, ranging from 0 to -(content_height - view_height)
        let scroll_y = -self.scroll_offset.y;
        let max_scroll_y = (content_height - view_height).max(px(0.0));
        
        let thumb_y = if max_scroll_y > px(0.0) {
            (scroll_y / max_scroll_y) * max_thumb_y
        } else {
            px(0.0)
        };

        Bounds::new(
            point(bounds.right() - scrollbar_width, bounds.top() + thumb_y),
            size(scrollbar_width, thumb_height),
        )
    }
}
