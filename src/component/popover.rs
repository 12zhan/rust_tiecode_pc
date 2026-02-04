use gpui::*;
use std::rc::Rc;

pub struct Popover {
    open: bool,
    position: Point<Pixels>,
    content: Option<AnyElement>,
    on_dismiss: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    dismiss_on_outside_click: bool,
    style: StyleRefinement,
}

/// A non-modal popup (popover) that is positioned relative to some anchor point.
///
/// Unlike `modal()`, it has no visible backdrop and is not centered.
#[track_caller]
pub fn popover() -> Popover {
    let style = StyleRefinement::default()
        .flex()
        .flex_col()
        .bg(rgb(0xff2d353b))
        .border_1()
        .border_color(rgb(0xff3c474d))
        .rounded_md()
        .p(px(12.0))
        .w(px(220.0));

    Popover {
        open: false,
        position: point(px(0.0), px(0.0)),
        content: None,
        on_dismiss: None,
        dismiss_on_outside_click: true,
        style,
    }
}

impl Popover {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the top-left position of the popover, usually based on an anchor's bounds.
    pub fn position(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.content = Some(child.into_any_element());
        self
    }

    pub fn on_dismiss(mut self, on_dismiss: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(on_dismiss));
        self
    }

    /// When true, clicking anywhere outside the popover triggers `on_dismiss` (if set).
    pub fn dismiss_on_outside_click(mut self, dismiss_on_outside_click: bool) -> Self {
        self.dismiss_on_outside_click = dismiss_on_outside_click;
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for Popover {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        if !self.open {
            return div().into_any_element();
        }

        let on_dismiss = self.on_dismiss.clone();
        let dismiss_on_outside_click = self.dismiss_on_outside_click;

        let mut layer = div().absolute().top(px(0.0)).left(px(0.0)).w_full().h_full();
        if dismiss_on_outside_click {
            let on_dismiss = on_dismiss.clone();
            layer = layer
                .occlude()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    if let Some(on_dismiss) = on_dismiss.as_ref() {
                        on_dismiss(window, cx);
                    }
                });
        }

        let position = self.position;
        let style = self.style;
        let mut panel = div().on_any_mouse_down(|_, _window, cx| cx.stop_propagation());
        *panel.style() = style;
        panel = panel.absolute().top(position.y).left(position.x);

        if let Some(content) = self.content {
            panel = panel.child(content);
        }

        layer.child(panel).into_any_element()
    }
}
