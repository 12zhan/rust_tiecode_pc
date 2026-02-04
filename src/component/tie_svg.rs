use gpui::*;

pub struct TieSvg {
    path: Option<SharedString>,
    transformation: Option<Transformation>,
    original_colors: bool,
    style: StyleRefinement,
}

/// A wrapper around GPUI's `svg()` element that can optionally keep the SVG's original colors.
///
/// - `original_colors(false)` (default): uses `svg()` (monochrome, color comes from `text_color`).
/// - `original_colors(true)`: uses `img()` to render the SVG with its original colors.
#[track_caller]
pub fn tie_svg() -> TieSvg {
    TieSvg {
        path: None,
        transformation: None,
        original_colors: false,
        style: StyleRefinement::default(),
    }
}

impl TieSvg {
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Render the SVG with its original colors (instead of tinting with `text_color`).
    pub fn original_colors(mut self, original_colors: bool) -> Self {
        self.original_colors = original_colors;
        self
    }

    /// Transform the SVG element with the given transformation.
    ///
    /// Note: this only applies when `original_colors(false)` (monochrome `svg()` path).
    pub fn with_transformation(mut self, transformation: Transformation) -> Self {
        self.transformation = Some(transformation);
        self
    }
}

impl Styled for TieSvg {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for TieSvg {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let TieSvg {
            path,
            transformation,
            original_colors,
            style,
        } = self;

        let Some(path) = path else {
            let mut element = div();
            *element.style() = style;
            return element.into_any_element();
        };

        if original_colors {
            let mut element = img(path);
            *element.style() = style;
            element.into_any_element()
        } else {
            let mut element = svg().path(path);
            if let Some(transformation) = transformation {
                element = element.with_transformation(transformation);
            }
            *element.style() = style;
            element.into_any_element()
        }
    }
}

