//! A widget for laying out multiple widgets in a similar fashion as how words
//! are wrapped in a paragraph.

use figures::units::UPx;
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, Size, Zero};
use intentional::Cast;

use crate::context::{AsEventContext, GraphicsContext, LayoutContext, Trackable};
use crate::styles::components::{IntrinsicPadding, LayoutOrder, VerticalAlignment};
use crate::styles::{FlexibleDimension, HorizontalOrder, VerticalAlign};
use crate::value::{IntoValue, Value};
use crate::widget::{Baseline, MountedChildren, Widget, WidgetLayout, WidgetList};
use crate::ConstraintLimit;

/// A widget that lays its children out horizontally, wrapping into multiple
/// rows when the widgets can't fit.
///
/// This widget is designed to mimic how text layout occurs for words within a
/// paragraph.
#[derive(Debug)]
pub struct Wrap {
    /// The children to wrap.
    pub children: Value<WidgetList>,
    /// The horizontal alignment for widgets on the same row.
    pub align: Value<WrapAlign>,
    /// The spacing to place between widgets. When [`FlexibleDimension::Auto`]
    /// is set, [`IntrinsicPadding`] will be used.
    pub spacing: Value<Size<FlexibleDimension>>,
    mounted: MountedChildren,
}

impl Wrap {
    /// Returns a new widget that wraps `children`.
    #[must_use]
    pub fn new(children: impl IntoValue<WidgetList>) -> Self {
        Self {
            children: children.into_value(),
            align: Value::default(),
            spacing: Value::Constant(Size::squared(FlexibleDimension::Auto)),
            mounted: MountedChildren::default(),
        }
    }

    /// Sets the spacing between widgets and returns self.
    #[must_use]
    pub fn spacing(mut self, spacing: impl IntoValue<Size<FlexibleDimension>>) -> Self {
        self.spacing = spacing.into_value();
        self
    }

    /// Sets the horizontal alignment and returns self.
    #[must_use]
    pub fn align(mut self, align: impl IntoValue<WrapAlign>) -> Self {
        self.align = align.into_value();
        self
    }

    fn horizontal_alignment(
        align: WrapAlign,
        order: HorizontalOrder,
        remaining: UPx,
        row_children_len: usize,
    ) -> (UPx, UPx) {
        match (align, order) {
            (WrapAlign::Start, HorizontalOrder::LeftToRight)
            | (WrapAlign::End, HorizontalOrder::RightToLeft) => (UPx::ZERO, UPx::ZERO),
            (WrapAlign::End, HorizontalOrder::LeftToRight)
            | (WrapAlign::Start, HorizontalOrder::RightToLeft) => (remaining, UPx::ZERO),
            (WrapAlign::Center, _) => (remaining / 2, UPx::ZERO),
            (WrapAlign::SpaceBetween, _) => {
                if row_children_len > 1 {
                    (UPx::ZERO, remaining / (row_children_len - 1).cast::<u32>())
                } else {
                    (UPx::ZERO, UPx::ZERO)
                }
            }
            (WrapAlign::SpaceEvenly, _) => {
                let spacing = remaining / row_children_len.cast::<u32>();
                (spacing / 2, spacing)
            }
            (WrapAlign::SpaceAround, _) => {
                let spacing = remaining / (row_children_len + 1).cast::<u32>();
                (spacing, spacing)
            }
        }
    }
}

impl Widget for Wrap {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        for child in self.mounted.children() {
            context.for_other(child).redraw();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        struct RowChild {
            index: usize,
            x: UPx,
            layout: WidgetLayout,
        }

        let order = context.get(&LayoutOrder).horizontal;

        self.children.invalidate_when_changed(context);
        let align = self.align.get_tracking_invalidate(context);
        let vertical_align = context.get(&VerticalAlignment);
        let spacing = self
            .spacing
            .get_tracking_invalidate(context)
            .map(|dimension| match dimension {
                FlexibleDimension::Auto => context.get(&IntrinsicPadding),
                FlexibleDimension::Dimension(dimension) => dimension,
            })
            .into_upx(context.gfx.scale())
            .round();
        self.mounted
            .synchronize_with(&self.children, &mut context.as_event_context());

        let mut y = UPx::ZERO;
        let mut row_children = Vec::new();
        let mut index = 0;
        let width = available_space.width.max();
        let child_constraints =
            available_space.map(|limit| ConstraintLimit::SizeToFit(limit.max()));
        let mut first_baseline = Baseline::NONE;
        while index < self.mounted.children().len() {
            if y != UPx::ZERO {
                y += spacing.height;
            }
            // Find all children that can fit on this next row.
            let mut x = UPx::ZERO;
            let mut max_height = UPx::ZERO;
            let mut max_baseline = Baseline::NONE;
            while let Some(child) = self.mounted.children().get(index) {
                let child_layout = context.for_other(child).layout(child_constraints);
                max_baseline = child_layout.baseline.max(max_baseline);
                max_height = max_height.max(child_layout.size.height);

                let child_x = if x.is_zero() {
                    x
                } else {
                    x.saturating_add(spacing.width)
                };
                let after_child = child_x.saturating_add(child_layout.size.width);

                if x > 0 && after_child > width {
                    break;
                }

                row_children.push(RowChild {
                    index,
                    x: child_x,
                    layout: child_layout,
                });

                x = after_child;
                index += 1;
            }

            // Calculate the horizontal alignment.
            let remaining = width.saturating_sub(x);
            let (x, space_between) = if remaining > 0 {
                Self::horizontal_alignment(align, order, remaining, row_children.len())
            } else {
                (UPx::ZERO, UPx::ZERO)
            };

            if y == 0 {
                first_baseline = max_baseline;
            }

            // Position the children
            let mut additional_x = x;
            for (child_index, child) in row_children.drain(..).enumerate() {
                if child_index > 0 {
                    additional_x += space_between;
                }
                let child_x = additional_x + child.x;
                let child_y = y + match vertical_align {
                    VerticalAlign::Top => UPx::ZERO,
                    VerticalAlign::Center => (max_height - child.layout.size.height) / 2,
                    VerticalAlign::Bottom => max_height - child.layout.size.height,
                };

                context.set_child_layout(
                    &self.mounted.children()[child.index],
                    Rect::new(Point::new(child_x, child_y), child.layout.size).into_signed(),
                );
            }

            y += max_height;
        }

        WidgetLayout {
            size: Size::new(width, y).into_unsigned(),
            baseline: first_baseline,
        }
    }
}

/// The horizontal alignment to apply to widgets inside of a [`Wrap`].
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum WrapAlign {
    /// Position the widgets at the start of the line, honoring [`LayoutOrder`].
    #[default]
    Start,
    /// Position the widgets at the end of the line, honoring [`LayoutOrder`].
    End,
    /// Position the widgets centered on the line.
    Center,
    /// Position the widgets evenly along the line with no space before the
    /// first widget or after the last widget.
    SpaceBetween,
    /// Position the widgets evenly along the line with half of the amount of
    /// spacing used between the widgets placed at the start and end of the
    /// line.
    SpaceEvenly,
    /// Position the widgets evenly along the line with an equal amount of
    /// spacing used between the widgets placed at the start and end of the
    /// line.
    SpaceAround,
}
