use std::fmt::Debug;

use kludgine::figures::units::UPx;
use kludgine::figures::{Fraction, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size};

use crate::context::{AsEventContext, GraphicsContext, LayoutContext};
use crate::styles::{Edges, FlexibleDimension};
use crate::value::{IntoValue, Value};
use crate::widget::{MakeWidget, Widget, WidgetRef};
use crate::ConstraintLimit;

/// A widget aligns its contents to its container's boundaries.
#[derive(Debug)]
pub struct Align {
    child: WidgetRef,
    edges: Value<Edges<FlexibleDimension>>,
}

impl Align {
    /// Returns a new spacing widget containing `widget`, surrounding it with
    /// `margin`.
    pub fn new(margin: impl IntoValue<Edges<FlexibleDimension>>, widget: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(widget),
            edges: margin.into_value(),
        }
    }

    /// Returns a new spacing widget that centers `widget` vertically and
    /// horizontally.
    pub fn centered(widget: impl MakeWidget) -> Self {
        Self::new(FlexibleDimension::Auto, widget)
    }

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Layout {
        let margin = self.edges.get();
        let vertical = FrameInfo::new(context.graphics.scale(), margin.top, margin.bottom);
        let horizontal = FrameInfo::new(context.graphics.scale(), margin.left, margin.right);

        let content_available = Size::new(
            horizontal.child_constraint(available_space.width),
            vertical.child_constraint(available_space.height),
        );

        let child = self.child.mounted(&mut context.as_event_context());
        let content_size = context.for_other(child).layout(content_available);

        let (left, right, width) = horizontal.measure(available_space.width, content_size.width);
        let (top, bottom, height) = vertical.measure(available_space.height, content_size.height);

        Layout {
            margin: Edges {
                left,
                right,
                top,
                bottom,
            },
            content: Size::new(width, height),
        }
    }
}

struct FrameInfo {
    a: Option<UPx>,
    b: Option<UPx>,
}

impl FrameInfo {
    fn new(scale: Fraction, a: FlexibleDimension, b: FlexibleDimension) -> Self {
        let a = match a {
            FlexibleDimension::Auto => None,
            FlexibleDimension::Dimension(dimension) => {
                Some(dimension.into_px(scale).into_unsigned())
            }
        };
        let b = match b {
            FlexibleDimension::Auto => None,
            FlexibleDimension::Dimension(dimension) => {
                Some(dimension.into_px(scale).into_unsigned())
            }
        };
        Self { a, b }
    }

    fn child_constraint(&self, available: ConstraintLimit) -> ConstraintLimit {
        match (self.a, self.b) {
            (Some(a), Some(b)) => available - (a + b),
            // If we have at least one auto-measurement, force the constraint
            // into ClippedAfter mode to make the widget attempt to size the
            // content to fit.
            (Some(one), None) | (None, Some(one)) => {
                ConstraintLimit::ClippedAfter(available.max() - one)
            }
            (None, None) => ConstraintLimit::ClippedAfter(available.max()),
        }
    }

    fn measure(&self, available: ConstraintLimit, content: UPx) -> (UPx, UPx, UPx) {
        match available {
            ConstraintLimit::Known(size) => {
                let remaining = size - content;
                let (a, b) = match (self.a, self.b) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) => (a, remaining - a),
                    (None, Some(b)) => (remaining - b, b),
                    (None, None) => {
                        let a = remaining / 2;
                        let b = remaining - a;
                        (a, b)
                    }
                };

                (a, b, size - a - b)
            }
            ConstraintLimit::ClippedAfter(_) => (
                self.a.unwrap_or_default(),
                self.b.unwrap_or_default(),
                content,
            ),
        }
    }
}

impl Widget for Align {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let child = self.child.mounted(&mut context.as_event_context());
        context.for_other(child).redraw();
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let child = self.child.mounted(&mut context.as_event_context());
        let layout = self.measure(available_space, context);
        context.set_child_layout(
            &child,
            Rect::new(
                Point::new(
                    layout.margin.left.into_signed(),
                    layout.margin.top.into_signed(),
                ),
                layout.content.into_signed(),
            ),
        );

        layout.size().into_unsigned()
    }
}

#[derive(Debug)]
struct Layout {
    margin: Edges<UPx>,
    content: Size<UPx>,
}

impl Layout {
    pub fn size(&self) -> Size<UPx> {
        self.margin.size() + self.content
    }
}
