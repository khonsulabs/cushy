//! A widget that combines a collection of [`Children`] widgets into one.
// TODO on scale change, all `Lp` children need to resize

use std::ops::{Bound, Deref};

use alot::{LotId, OrderedLots};
use kludgine::figures::units::{Lp, UPx};
use kludgine::figures::{Fraction, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext};
use crate::styles::Dimension;
use crate::value::{Generation, IntoValue, Value};
use crate::widget::{Children, ManagedWidget, Widget, WidgetRef};
use crate::widgets::{Expand, Resize};
use crate::ConstraintLimit;

/// A widget that displays a collection of [`Children`] widgets in a
/// [direction](StackDirection).
#[derive(Debug)]
pub struct Stack {
    /// The direction to display the children using.
    pub direction: Value<StackDirection>,
    /// The children widgets that belong to this array.
    pub children: Value<Children>,
    layout: Layout,
    layout_generation: Option<Generation>,
    // TODO Refactor synced_children into its own type.
    synced_children: Vec<ManagedWidget>,
}

impl Stack {
    /// Returns a new widget with the given direction and widgets.
    pub fn new(
        direction: impl IntoValue<StackDirection>,
        widgets: impl IntoValue<Children>,
    ) -> Self {
        let direction = direction.into_value();

        let initial_direction = direction.get();

        Self {
            direction,
            children: widgets.into_value(),
            layout: Layout::new(initial_direction),
            layout_generation: None,
            synced_children: Vec::new(),
        }
    }

    /// Returns a new instance that displays `widgets` in a series of columns.
    pub fn columns(widgets: impl IntoValue<Children>) -> Self {
        Self::new(StackDirection::columns(), widgets)
    }

    /// Returns a new instance that displays `widgets` in a series of rows.
    pub fn rows(widgets: impl IntoValue<Children>) -> Self {
        Self::new(StackDirection::rows(), widgets)
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_, '_>) {
        let current_generation = self.children.generation();
        if current_generation.map_or_else(
            || self.children.map(Children::len) != self.layout.children.len(),
            |gen| Some(gen) != self.layout_generation,
        ) {
            self.layout_generation = self.children.generation();
            self.children.map(|children| {
                for (index, widget) in children.iter().enumerate() {
                    if self
                        .synced_children
                        .get(index)
                        .map_or(true, |child| child != widget)
                    {
                        // These entries do not match. See if we can find the
                        // new id somewhere else, if so we can swap the entries.
                        if let Some((swap_index, _)) = self
                            .synced_children
                            .iter()
                            .enumerate()
                            .skip(index + 1)
                            .find(|(_, child)| *child == widget)
                        {
                            self.synced_children.swap(index, swap_index);
                            self.layout.swap(index, swap_index);
                        } else {
                            // This is a brand new child.
                            let guard = widget.lock();
                            let (mut widget, dimension) = if let Some((weight, expand)) = guard
                                .downcast_ref::<Expand>()
                                .and_then(|expand| expand.weight().map(|weight| (weight, expand)))
                            {
                                (
                                    expand.child().clone(),
                                    StackDimension::Fractional { weight },
                                )
                            } else if let Some((child, size)) =
                                guard.downcast_ref::<Resize>().and_then(|r| {
                                    let range = match self.layout.orientation.orientation {
                                        StackOrientation::Row => r.height,
                                        StackOrientation::Column => r.width,
                                    };
                                    range.minimum().map(|min| {
                                        (
                                            r.child().clone(),
                                            StackDimension::Measured {
                                                min,
                                                _max: range.end,
                                            },
                                        )
                                    })
                                })
                            {
                                (child, size)
                            } else {
                                (
                                    WidgetRef::Unmounted(widget.clone()),
                                    StackDimension::FitContent,
                                )
                            };
                            drop(guard);
                            self.synced_children.insert(index, widget.mounted(context));

                            self.layout
                                .insert(index, dimension, context.kludgine.scale());
                        }
                    }
                }

                // Any children remaining at the end of this process are ones
                // that have been removed.
                for removed in self.synced_children.drain(children.len()..) {
                    context.remove_child(&removed);
                }
                self.layout.truncate(children.len());
            });
        }
    }
}

impl Widget for Stack {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        for (layout, child) in self.layout.iter().zip(&self.synced_children) {
            if layout.size > 0 {
                context.for_other(child).redraw();
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        self.synchronize_children(&mut context.as_event_context());

        let content_size = self.layout.update(
            available_space,
            context.gfx.scale(),
            |child_index, constraints, persist| {
                let mut context = context.for_other(&self.synced_children[child_index]);
                if !persist {
                    context = context.as_temporary();
                }
                context.layout(constraints)
            },
        );

        for (layout, child) in self.layout.iter().zip(&self.synced_children) {
            if layout.size > 0 {
                context.set_child_layout(
                    child,
                    Rect::new(
                        self.layout
                            .orientation
                            .make_point(layout.offset, UPx(0))
                            .into_signed(),
                        self.layout
                            .orientation
                            .make_size(layout.size, self.layout.other)
                            .into_signed(),
                    ),
                );
            }
        }

        content_size
    }
}

/// The direction of an [`Stack`] widget.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StackDirection {
    /// The orientation of the widgets.
    pub orientation: StackOrientation,
    /// If true, the widgets will be laid out in reverse order.
    pub reverse: bool,
}

impl StackDirection {
    /// Display child widgets as columns.
    #[must_use]
    pub const fn columns() -> Self {
        Self {
            orientation: StackOrientation::Column,
            reverse: false,
        }
    }

    /// Display child widgets as columns in reverse order.
    #[must_use]
    pub const fn columns_rev() -> Self {
        Self {
            orientation: StackOrientation::Column,
            reverse: true,
        }
    }

    /// Display child widgets as rows.
    #[must_use]
    pub const fn rows() -> Self {
        Self {
            orientation: StackOrientation::Row,
            reverse: false,
        }
    }

    /// Display child widgets as rows in reverse order.
    #[must_use]
    pub const fn rows_rev() -> Self {
        Self {
            orientation: StackOrientation::Row,
            reverse: true,
        }
    }

    /// Splits a size into its measured and other parts.
    pub(crate) fn split_size<U>(self, s: Size<U>) -> (U, U) {
        match self.orientation {
            StackOrientation::Row => (s.height, s.width),
            StackOrientation::Column => (s.width, s.height),
        }
    }

    /// Combines split values into a [`Size`].
    pub(crate) fn make_size<U>(self, measured: U, other: U) -> Size<U> {
        match self.orientation {
            StackOrientation::Row => Size::new(other, measured),
            StackOrientation::Column => Size::new(measured, other),
        }
    }

    /// Combines split values into a [`Point`].
    pub(crate) fn make_point<U>(self, measured: U, other: U) -> Point<U> {
        match self.orientation {
            StackOrientation::Row => Point::new(other, measured),
            StackOrientation::Column => Point::new(measured, other),
        }
    }
}

/// The orientation (Row/Column) of an [`Stack`] widget.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

pub enum StackOrientation {
    /// The child widgets should be displayed as rows.
    Row,
    /// The child widgets should be displayed as columns.
    Column,
}

/// The strategy to use when laying a widget out inside of an [`Stack`].
#[derive(Debug, Clone, Copy)]
enum StackDimension {
    /// Attempt to lay out the widget based on its contents.
    FitContent,
    /// Use a fractional amount of the available space.
    Fractional {
        /// The weight to apply to this widget when dividing multiple widgets
        /// fractionally.
        weight: u8,
    },
    /// Use a range for this widget's size.
    Measured {
        /// The minimum size for the widget.
        min: Dimension,
        /// The optional maximum size for the widget.
        _max: Bound<Dimension>,
    },
}

#[derive(Debug)]
struct Layout {
    children: OrderedLots<StackDimension>,
    layouts: Vec<StackLayout>,
    pub other: UPx,
    total_weights: u32,
    allocated_space: (UPx, Lp),
    fractional: Vec<(LotId, u8)>,
    measured: Vec<LotId>,
    pub orientation: StackDirection,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct StackLayout {
    pub offset: UPx,
    pub size: UPx,
}

impl Layout {
    pub const fn new(orientation: StackDirection) -> Self {
        Self {
            orientation,
            children: OrderedLots::new(),
            layouts: Vec::new(),
            other: UPx(0),
            total_weights: 0,
            allocated_space: (UPx(0), Lp(0)),
            fractional: Vec::new(),
            measured: Vec::new(),
        }
    }

    #[cfg(test)] // only used in testing
    pub fn push(&mut self, child: StackDimension, scale: Fraction) {
        self.insert(self.len(), child, scale);
    }

    pub fn remove(&mut self, index: usize) -> StackDimension {
        let (id, dimension) = self.children.remove_by_index(index).expect("invalid index");
        self.layouts.remove(index);

        match dimension {
            StackDimension::FitContent => {
                self.measured.retain(|&measured| measured != id);
            }
            StackDimension::Fractional { weight } => {
                self.fractional.retain(|(measured, _)| *measured != id);
                self.total_weights -= u32::from(weight);
            }
            StackDimension::Measured { min, .. } => match min {
                Dimension::Px(pixels) => {
                    self.allocated_space.0 -= pixels.into_unsigned();
                }
                Dimension::Lp(lp) => {
                    self.allocated_space.1 -= lp;
                }
            },
        }

        dimension
    }

    pub fn truncate(&mut self, new_length: usize) {
        while self.len() > new_length {
            self.remove(self.len() - 1);
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        self.children.swap(a, b);
    }

    pub fn insert(&mut self, index: usize, child: StackDimension, scale: Fraction) {
        let id = self.children.insert(index, child);
        let layout = match child {
            StackDimension::FitContent => {
                self.measured.push(id);
                UPx(0)
            }
            StackDimension::Fractional { weight } => {
                self.total_weights += u32::from(weight);
                self.fractional.push((id, weight));
                UPx(0)
            }
            StackDimension::Measured { min, .. } => {
                match min {
                    Dimension::Px(size) => self.allocated_space.0 += size.into_unsigned(),
                    Dimension::Lp(size) => self.allocated_space.1 += size,
                }
                min.into_px(scale).into_unsigned()
            }
        };
        self.layouts.insert(
            index,
            StackLayout {
                offset: UPx(0),
                size: layout,
            },
        );
    }

    pub fn update(
        &mut self,
        available: Size<ConstraintLimit>,
        scale: Fraction,
        mut measure: impl FnMut(usize, Size<ConstraintLimit>, bool) -> Size<UPx>,
    ) -> Size<UPx> {
        let (space_constraint, other_constraint) = self.orientation.split_size(available);
        let available_space = space_constraint.max();
        let allocated_space =
            self.allocated_space.0 + self.allocated_space.1.into_px(scale).into_unsigned();
        let mut remaining = available_space.saturating_sub(allocated_space);

        // Measure the children that fit their content
        for &id in &self.measured {
            let index = self.children.index_of_id(id).expect("child not found");
            let (measured, _) = self.orientation.split_size(measure(
                index,
                self.orientation
                    .make_size(ConstraintLimit::ClippedAfter(remaining), other_constraint),
                false,
            ));
            self.layouts[index].size = measured;
            remaining = remaining.saturating_sub(measured);
        }

        // Measure the weighted children within the remaining space
        if self.total_weights > 0 {
            let space_per_weight = remaining / self.total_weights;
            remaining %= self.total_weights;
            for (fractional_index, &(id, weight)) in self.fractional.iter().enumerate() {
                let index = self.children.index_of_id(id).expect("child not found");
                let mut size = space_per_weight * u32::from(weight);

                // If we have fractional amounts remaining, divide the pixels
                if remaining > 0 {
                    let from_end = u32::try_from(self.fractional.len() - fractional_index)
                        .expect("too many items");
                    if remaining >= from_end {
                        let amount = (remaining + from_end - 1) / from_end;
                        remaining -= amount;
                        size += amount;
                    }
                }

                self.layouts[index].size = size;
            }
        }

        // Now that we know the constrained sizes, we can measure the children
        // to get the other measurement using the constrainted measurement.
        self.other = UPx(0);
        let mut offset = UPx(0);
        for index in 0..self.children.len() {
            self.layouts[index].offset = offset;
            offset += self.layouts[index].size;
            let (_, measured) = self.orientation.split_size(measure(
                index,
                self.orientation.make_size(
                    ConstraintLimit::Known(self.layouts[index].size.into_px(scale).into_unsigned()),
                    other_constraint,
                ),
                false,
            ));
            self.other = self.other.max(measured);
        }

        self.other = match other_constraint {
            ConstraintLimit::Known(max) => self.other.max(max),
            ConstraintLimit::ClippedAfter(clip_limit) => self.other.min(clip_limit),
        };

        // Finally layout the widgets with the final constraints
        for index in 0..self.children.len() {
            self.orientation.split_size(measure(
                index,
                self.orientation.make_size(
                    ConstraintLimit::Known(self.layouts[index].size.into_px(scale).into_unsigned()),
                    ConstraintLimit::Known(self.other),
                ),
                true,
            ));
        }

        self.orientation.make_size(offset, self.other)
    }
}

impl Deref for Layout {
    type Target = [StackLayout];

    fn deref(&self) -> &Self::Target {
        &self.layouts
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::ops::Bound;

    use kludgine::figures::units::UPx;
    use kludgine::figures::{Fraction, IntoSigned, Size};

    use super::{Layout, StackDimension, StackDirection};
    use crate::styles::Dimension;
    use crate::ConstraintLimit;

    struct Child {
        size: UPx,
        dimension: StackDimension,
        other: UPx,
        divisible_by: Option<UPx>,
    }

    impl Child {
        pub fn new(size: impl Into<UPx>, other: impl Into<UPx>) -> Self {
            Self {
                size: size.into(),
                dimension: StackDimension::FitContent,
                other: other.into(),
                divisible_by: None,
            }
        }

        pub fn fixed_size(mut self, size: UPx) -> Self {
            self.dimension = StackDimension::Measured {
                min: Dimension::Px(size.into_signed()),
                _max: Bound::Unbounded,
            };
            self
        }

        pub fn weighted(mut self, weight: u8) -> Self {
            self.dimension = StackDimension::Fractional { weight };
            self
        }

        pub fn divisible_by(mut self, split_at: impl Into<UPx>) -> Self {
            self.divisible_by = Some(split_at.into());
            self
        }
    }

    fn assert_measured_children_in_orientation(
        orientation: StackDirection,
        children: &[Child],
        available: Size<ConstraintLimit>,
        expected: &[UPx],
        expected_size: Size<UPx>,
    ) {
        assert_eq!(children.len(), expected.len());
        let mut flex = Layout::new(orientation);
        for child in children {
            flex.push(child.dimension, Fraction::ONE);
        }

        let computed_size =
            flex.update(available, Fraction::ONE, |index, constraints, _persist| {
                let (measured_constraint, _other_constraint) = orientation.split_size(constraints);
                let child = &children[index];
                let maximum_measured = measured_constraint.max();
                let (measured, other) =
                    match (child.size.cmp(&maximum_measured), child.divisible_by) {
                        (Ordering::Greater, Some(divisible_by)) => {
                            let available_divided = maximum_measured / divisible_by;
                            let rows = ((child.size + divisible_by - 1) / divisible_by
                                + available_divided
                                - 1)
                                / available_divided;
                            (available_divided * divisible_by, child.other * rows)
                        }
                        _ => (child.size, child.other),
                    };
                orientation.make_size(measured, other)
            });
        assert_eq!(computed_size, expected_size);
        let mut offset = UPx(0);
        for ((index, &child), &expected) in flex.iter().enumerate().zip(expected) {
            assert_eq!(
                child.size,
                expected,
                "child {index} measured to {}, expected {}",
                child.size,
                expected // TODO Display for UPx
            );
            assert_eq!(child.offset, offset);
            offset += child.size;
        }
    }

    fn assert_measured_children(
        children: &[Child],
        main_constraint: ConstraintLimit,
        other_constraint: ConstraintLimit,
        expected: &[UPx],
        expected_measured: UPx,
        expected_other: UPx,
    ) {
        assert_measured_children_in_orientation(
            StackDirection::rows(),
            children,
            StackDirection::rows().make_size(main_constraint, other_constraint),
            expected,
            StackDirection::rows().make_size(expected_measured, expected_other),
        );
        assert_measured_children_in_orientation(
            StackDirection::columns(),
            children,
            StackDirection::columns().make_size(main_constraint, other_constraint),
            expected,
            StackDirection::columns().make_size(expected_measured, expected_other),
        );
    }

    #[test]
    fn size_to_fit() {
        assert_measured_children(
            &[Child::new(3, 1), Child::new(3, 1), Child::new(3, 1)],
            ConstraintLimit::ClippedAfter(UPx(10)),
            ConstraintLimit::ClippedAfter(UPx(10)),
            &[UPx(3), UPx(3), UPx(3)],
            UPx(9),
            UPx(1),
        );
    }

    #[test]
    fn wrapping() {
        // This tests some fun rounding edge cases. Because the total weights is
        // 4 and the size is 10, we have inexact math to determine the pixel
        // width of each child.
        //
        // In this particular example, it shows the weights are clamped so that
        // each is credited for 2px. This is why the first child ends up with
        // 4px. However, with 4 total weight, that leaves a remaining 2px to be
        // assigned. The flex algorithm divides the remaining pixels amongst the
        // remaining children.
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Known(UPx(10)),
            ConstraintLimit::ClippedAfter(UPx(10)),
            &[UPx(4), UPx(3), UPx(3)],
            UPx(10),
            UPx(7), // 20 / 3 = 6.666, rounded up is 7
        );
        // Same as above, but with an 11px box. This creates a leftover of 3 px
        // (11 % 4), adding 1px to all three children.
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Known(UPx(11)),
            ConstraintLimit::ClippedAfter(UPx(11)),
            &[UPx(5), UPx(3), UPx(3)],
            UPx(11),
            UPx(7), // 20 / 3 = 6.666, rounded up is 7
        );
        // 12px box. This creates no leftover.
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Known(UPx(12)),
            ConstraintLimit::ClippedAfter(UPx(12)),
            &[UPx(6), UPx(3), UPx(3)],
            UPx(12),
            UPx(4), // 20 / 6 = 3.666, rounded up is 4
        );
        // 13px box. This creates a leftover of 1 px (13 % 4), adding 1px only
        // to the final child
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Known(UPx(13)),
            ConstraintLimit::ClippedAfter(UPx(13)),
            &[UPx(6), UPx(3), UPx(4)],
            UPx(13),
            UPx(4), // 20 / 6 = 3.666, rounded up is 4
        );
    }

    #[test]
    fn fixed_size() {
        assert_measured_children(
            &[
                Child::new(3, 1).fixed_size(UPx(7)),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Known(UPx(15)),
            ConstraintLimit::ClippedAfter(UPx(15)),
            &[UPx(7), UPx(4), UPx(4)],
            UPx(15),
            UPx(1),
        );
    }
}
