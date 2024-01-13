//! A visual delimiter widget.

use figures::units::{Lp, UPx};
use figures::{Point, ScreenScale, Size};
use kludgine::shapes::{PathBuilder, StrokeOptions};
use kludgine::Color;

use crate::context::{GraphicsContext, LayoutContext};
use crate::styles::components::TextColor;
use crate::styles::{Dimension, FlexibleDimension};
use crate::value::{IntoValue, Value};
use crate::widget::Widget;
use crate::ConstraintLimit;

#[derive(Debug)]
enum Orientation {
    Horizontal,
    Vertical,
}

/// A visual delimiter that can be horizontal or vertical.
///
/// This is similar to html's `<hr>` tag.
#[derive(Debug)]
pub struct Delimiter {
    size: Value<FlexibleDimension>,
    orientation: Orientation,
}

impl Default for Delimiter {
    fn default() -> Self {
        Self::horizontal()
    }
}

impl Delimiter {
    fn new(orientation: Orientation) -> Self {
        Self {
            size: Value::Constant(FlexibleDimension::Auto),
            orientation,
        }
    }

    /// Returns a horizontal delimiter.
    #[must_use]
    pub fn horizontal() -> Self {
        Self::new(Orientation::Horizontal)
    }

    /// Returns a vertical delimiter.
    #[must_use]
    pub fn vertical() -> Self {
        Self::new(Orientation::Vertical)
    }

    /// Sets the size of the delimiter.
    ///
    /// If auto, a theme-derived size is used.
    #[must_use]
    pub fn size(mut self, size: impl IntoValue<FlexibleDimension>) -> Self {
        self.size = size.into_value();
        self
    }

    fn get_size(&self, context: &mut GraphicsContext<'_, '_, '_, '_>) -> Dimension {
        match self.size.get_tracking_invalidate(context) {
            FlexibleDimension::Auto => context.get(&DelimiterSize),
            FlexibleDimension::Dimension(dimension) => dimension,
        }
    }
}

impl Widget for Delimiter {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let line_width = self.get_size(context).into_upx(context.gfx.scale());
        let half_line = line_width / 2;
        let end = match self.orientation {
            Orientation::Horizontal => Point::new(context.gfx.size().width - half_line, half_line),
            Orientation::Vertical => Point::new(half_line, context.gfx.size().height - half_line),
        };
        let color = context.get(&DelimiterColor);
        context.gfx.draw_shape(
            &PathBuilder::new(Point::squared(half_line))
                .line_to(end)
                .build()
                .stroke(StrokeOptions {
                    color,
                    line_width,
                    ..StrokeOptions::default()
                }),
        );
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let size = self.get_size(context).into_upx(context.gfx.scale());
        match self.orientation {
            Orientation::Horizontal => Size::new(available_space.width.max(), size),
            Orientation::Vertical => Size::new(size, available_space.height.max()),
        }
    }
}

define_components! {
    Delimiter {
        /// The [`Dimension`] to use as the size of a [`Delimiter`] widget.
        DelimiterSize(Dimension, "size", Dimension::Lp(Lp::new(2)))
        /// The [`Color`] draw a [`Delimiter`] widget using.
        DelimiterColor(Color, "color", @TextColor)
    }
}
