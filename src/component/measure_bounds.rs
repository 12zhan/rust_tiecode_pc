use gpui::*;

/// Wrap a single child element and report its bounds (window coordinates) during prepaint.
///
/// Useful for anchoring popovers/dropdowns to an element.
pub fn measure_bounds(
    child: impl IntoElement,
    on_bounds: impl Fn(Bounds<Pixels>, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .on_children_prepainted(move |bounds, window, cx| {
            if let Some(bounds) = bounds.first().copied() {
                on_bounds(bounds, window, cx);
            }
        })
        .child(child)
}

