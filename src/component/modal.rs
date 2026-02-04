use gpui::*;
use std::rc::Rc;

pub struct Modal {
    open: bool,
    title: Option<SharedString>,
    body: Option<AnyElement>,
    footer: Option<AnyElement>,
    on_dismiss: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    dismiss_on_backdrop: bool,
    show_close_button: bool,
    backdrop_color: Hsla,
    style: StyleRefinement,
}

#[track_caller]
pub fn modal() -> Modal {
    let style = StyleRefinement::default()
        .flex()
        .flex_col()
        .bg(rgb(0xff2d353b))
        .border_1()
        .border_color(rgb(0xff3c474d))
        .rounded_md()
        .p(px(16.0))
        .w(px(420.0));

    Modal {
        open: false,
        title: None,
        body: None,
        footer: None,
        on_dismiss: None,
        dismiss_on_backdrop: true,
        show_close_button: true,
        backdrop_color: rgba(0x00000080).into(),
        style,
    }
}

impl Modal {
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.body = Some(child.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn on_dismiss(mut self, on_dismiss: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(on_dismiss));
        self
    }

    pub fn dismiss_on_backdrop(mut self, dismiss_on_backdrop: bool) -> Self {
        self.dismiss_on_backdrop = dismiss_on_backdrop;
        self
    }

    pub fn show_close_button(mut self, show_close_button: bool) -> Self {
        self.show_close_button = show_close_button;
        self
    }

    pub fn backdrop_color(mut self, color: impl Into<Hsla>) -> Self {
        self.backdrop_color = color.into();
        self
    }
}

impl Styled for Modal {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for Modal {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        if !self.open {
            return div().into_any_element();
        }

        let on_dismiss = self.on_dismiss.clone();
        let dismiss_on_backdrop = self.dismiss_on_backdrop;

        let mut backdrop = div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w_full()
            .h_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .p(px(24.0))
            .bg(self.backdrop_color)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                if dismiss_on_backdrop {
                    if let Some(on_dismiss) = on_dismiss.as_ref() {
                        on_dismiss(window, cx);
                    }
                }
            });

        let on_dismiss_for_close = self.on_dismiss.clone();
        let show_close_button = self.show_close_button && on_dismiss_for_close.is_some();
        let title = self.title.clone();
        let style = self.style;

        let mut panel = div().on_any_mouse_down(|_, _window, cx| cx.stop_propagation());
        *panel.style() = style;

        if title.is_some() || show_close_button {
            let mut header = div()
                .flex()
                .items_center()
                .justify_between()
                .mb(px(12.0));

            if let Some(title) = title {
                header = header.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(0xffe6e0d9))
                        .child(title),
                );
            } else {
                header = header.child(div());
            }

            if show_close_button {
                let on_dismiss = on_dismiss_for_close.clone().expect("checked above");
                header = header.child(
                    div()
                        .cursor_pointer()
                        .text_size(px(16.0))
                        .text_color(rgb(0xffa9b1b6))
                        .hover(|s| s.text_color(rgb(0xffe6e0d9)))
                        .child("×")
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            on_dismiss(window, cx);
                        }),
                );
            }

            panel = panel.child(header);
        }

        if let Some(body) = self.body {
            panel = panel.child(body);
        }

        if let Some(footer) = self.footer {
            panel = panel.child(div().mt(px(12.0)).child(footer));
        }

        backdrop = backdrop.child(panel);
        backdrop.into_any_element()
    }
}
