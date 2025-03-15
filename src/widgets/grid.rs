//! A Widget that arranges children into rows and columns.
// TODO on scale change, all `Lp` children need to resize

use std::array;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use alot::{LotId, OrderedLots};
use figures::units::{Lp, UPx};
use figures::{Fraction, IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, Size, Zero};
use intentional::{Assert, Cast};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, Trackable};
use crate::reactive::value::{Generation, IntoValue, Value};
use crate::styles::components::IntrinsicPadding;
use crate::styles::Dimension;
use crate::widget::{Baseline, MakeWidget, MountedWidget, Widget, WidgetInstance, WidgetLayout};
use crate::ConstraintLimit;

/// A 2D grid of widgets.
#[derive(Debug)]
pub struct Grid<const ELEMENTS: usize> {
    columns: Value<[GridDimension; ELEMENTS]>,
    rows: Value<GridWidgets<ELEMENTS>>,
    live_rows: Vec<[MountedWidget; ELEMENTS]>,
    layout: GridLayout,
    layout_generation: Option<Generation>,
    spec_generation: Option<Generation>,
}

impl<const ELEMENTS: usize> Grid<ELEMENTS> {
    fn new(orientation: Orientation, rows: impl IntoValue<GridWidgets<ELEMENTS>>) -> Self {
        Self {
            columns: Value::Constant(array::from_fn(|_| GridDimension::FitContent)),
            rows: rows.into_value(),
            live_rows: Vec::new(),
            layout: GridLayout::new(orientation),
            layout_generation: None,
            spec_generation: None,
        }
    }

    /// Returns a grid that displays a list of rows of columns. The columns will
    /// share dimensions, while each row will be measured individually.
    #[must_use]
    pub fn from_rows(rows: impl IntoValue<GridWidgets<ELEMENTS>>) -> Self {
        Self::new(Orientation::Column, rows)
    }

    /// Returns a grid that displays a list of columns of rows. The rows will
    /// share dimensions, while each column will be measured individually.
    #[must_use]
    pub fn from_columns(columns: impl IntoValue<GridWidgets<ELEMENTS>>) -> Self {
        Self::new(Orientation::Row, columns)
    }

    /// Sets the dimensions for this grid and returns self.
    ///
    /// A grid is a 2d collection that orients itself either around rows or
    /// columns. If this grid was created using [`Self::from_rows()`],
    /// `dimensions` will control how the columns are measured. If this grid was
    /// created using [`Self::from_columns()`], `dimensions` will control how
    /// the rows are measured.
    #[must_use]
    pub fn dimensions(mut self, dimensions: impl IntoValue<[GridDimension; ELEMENTS]>) -> Self {
        self.columns = dimensions.into_value();
        self
    }

    fn synchronize_specs(&mut self, context: &mut EventContext<'_>) {
        let current_generation = self.columns.generation();
        let count_changed = self.layout.children.len() != ELEMENTS;
        if count_changed
            || current_generation.map_or_else(|| true, |gen| Some(gen) != self.spec_generation)
        {
            self.spec_generation = current_generation;
            self.columns.map(|columns| {
                self.layout.truncate(0);

                for (index, column) in columns.iter().enumerate() {
                    self.layout.insert(index, *column, context.kludgine.scale());
                }
            });
        }
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_>) {
        self.synchronize_specs(context);
        let current_generation = self.rows.generation();
        self.rows.invalidate_when_changed(context);
        if current_generation.map_or_else(
            || self.rows.map(|rows| rows.len()) != self.live_rows.len(),
            |gen| Some(gen) != self.layout_generation,
        ) {
            self.layout_generation = current_generation;
            self.rows.map(|rows| {
                self.layout.set_element_count(rows.len());
                for (index, row) in rows.iter().enumerate() {
                    if self.live_rows.get(index).map_or(true, |child| {
                        child.iter().zip(row.iter()).any(|(a, b)| a != b)
                    }) {
                        // These entries do not match. See if we can find the
                        // new id somewhere else, if so we can swap the entries.
                        if let Some((swap_index, _)) = self
                            .live_rows
                            .iter()
                            .enumerate()
                            .skip(index + 1)
                            .find(|(_, child)| child.iter().zip(row.iter()).all(|(a, b)| a == b))
                        {
                            self.live_rows.swap(index, swap_index);
                            self.layout.swap(index, swap_index);
                        } else {
                            self.live_rows.insert(
                                index,
                                array::from_fn(|index| context.push_child(row[index].clone())),
                            );
                        }
                    }
                }

                // Any children remaining at the end of this process are ones
                // that have been removed.
                for removed in self.live_rows.drain(rows.len()..) {
                    for removed in removed {
                        context.remove_child(&removed);
                    }
                }
            });
        }
    }
}

impl<const COLUMNS: usize> Widget for Grid<COLUMNS> {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        for (row, widgets) in self.live_rows.iter_mut().enumerate() {
            if self.layout.others[row] > 0 {
                for cell in widgets.iter() {
                    context.for_other(cell).redraw();
                }
            }
        }
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        for row in &mut self.live_rows {
            for col in row {
                col.remount_if_needed(context);
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        self.synchronize_children(&mut context.as_event_context());

        let content_layout = self.layout.update(
            available_space,
            context
                .get(&IntrinsicPadding)
                .into_upx(context.gfx.scale())
                .round(),
            context.gfx.scale(),
            |row, column, constraints, persist| {
                let mut context = context.for_other(&self.live_rows[column][row]);
                if !persist {
                    context = context.as_temporary();
                }
                context.layout(constraints)
            },
        );

        let mut other_offset = UPx::ZERO;
        for (&other_size, row) in self.layout.others.iter().zip(&self.live_rows) {
            if other_size > 0 {
                for (layout, cell) in self.layout.iter().zip(row) {
                    context.set_child_layout(
                        cell,
                        Rect::new(
                            self.layout
                                .orientation
                                .make_point(layout.offset, other_offset)
                                .into_signed(),
                            self.layout
                                .orientation
                                .make_size(layout.size, other_size)
                                .into_signed(),
                        ),
                    );
                }
                other_offset = other_offset.saturating_add(other_size);
            }
        }

        content_layout
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Grid")
            .field("dimensions", &self.columns)
            .field("entries", &self.rows)
            .finish()
    }
}

/// The orientation (Row/Column) of an [`Grid`] or
/// [`Stack`](crate::widgets::Stack) widget.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

pub enum Orientation {
    /// The child widgets should be displayed as rows.
    Row,
    /// The child widgets should be displayed as columns.
    Column,
}

impl Orientation {
    /// Splits a size into its measured and other parts.
    pub(crate) fn split_size<U>(self, s: Size<U>) -> (U, U) {
        match self {
            Orientation::Row => (s.height, s.width),
            Orientation::Column => (s.width, s.height),
        }
    }

    /// Combines split values into a [`Size`].
    pub(crate) fn make_size<U>(self, measured: U, other: U) -> Size<U> {
        match self {
            Orientation::Row => Size::new(other, measured),
            Orientation::Column => Size::new(measured, other),
        }
    }

    /// Combines split values into a [`Point`].
    pub(crate) fn make_point<U>(self, measured: U, other: U) -> Point<U> {
        match self {
            Orientation::Row => Point::new(other, measured),
            Orientation::Column => Point::new(measured, other),
        }
    }
}

/// The strategy to use when laying a widget out inside of an [`Grid`] or
/// [`Stack`](crate::widgets::Stack).
#[derive(Default, Debug, Clone, Copy)]
pub enum GridDimension {
    /// Attempt to lay out the widget based on its contents.
    #[default]
    FitContent,
    /// Use a fractional amount of the available space.
    Fractional {
        /// The weight to apply to this widget when dividing multiple widgets
        /// fractionally.
        weight: u8,
    },
    /// Use a specified size for the widget.
    Measured {
        /// The size for the widget.
        size: Dimension,
    },
}

#[derive(Debug)]
pub(crate) struct GridLayout {
    children: OrderedLots<GridDimension>,
    layouts: Vec<StackLayout>,
    pub elements_per_child: usize,
    pub others: Vec<UPx>,
    total_weights: u32,
    allocated_space: (UPx, Lp),
    fractional: Vec<(LotId, u8)>,
    fit_to_content: Vec<LotId>,
    premeasured: Vec<LotId>,
    measured_scale: Fraction,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct StackLayout {
    pub offset: UPx,
    pub baselines: Vec<Baseline>,
    pub size: UPx,
}

impl StackLayout {
    fn reset_baselines(&mut self, elements: usize) {
        self.baselines.clear();
        self.baselines.resize(elements, Baseline::NONE);
    }
}

impl GridLayout {
    pub fn new(orientation: Orientation) -> Self {
        Self {
            orientation,
            children: OrderedLots::new(),
            layouts: Vec::new(),
            elements_per_child: 1,
            others: vec![UPx::ZERO],
            total_weights: 0,
            allocated_space: (UPx::ZERO, Lp::ZERO),
            fractional: Vec::new(),
            fit_to_content: Vec::new(),
            premeasured: Vec::new(),
            measured_scale: Fraction::ONE,
        }
    }

    pub fn set_element_count(&mut self, count: usize) {
        self.others.resize(count, UPx::ZERO);
        self.elements_per_child = count;
    }

    #[cfg(test)] // only used in testing
    pub fn push(&mut self, child: GridDimension, scale: Fraction) {
        self.insert(self.len(), child, scale);
    }

    pub fn remove(&mut self, index: usize) -> GridDimension {
        let (id, dimension) = self.children.remove_by_index(index).expect("invalid index");
        self.layouts.remove(index);

        match dimension {
            GridDimension::FitContent => {
                self.fit_to_content.retain(|&measured| measured != id);
            }
            GridDimension::Fractional { weight } => {
                self.fractional.retain(|(measured, _)| *measured != id);
                self.total_weights -= u32::from(weight);
            }
            GridDimension::Measured { size: min, .. } => {
                self.premeasured.retain(|&measured| measured != id);
                match min {
                    Dimension::Px(pixels) => {
                        self.allocated_space.0 -= pixels.into_unsigned().ceil();
                    }
                    Dimension::Lp(lp) => {
                        self.allocated_space.1 -= lp;
                    }
                }
            }
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

    pub fn insert(&mut self, index: usize, child: GridDimension, scale: Fraction) {
        let id = self.children.insert(index, child);
        let layout = match child {
            GridDimension::FitContent => {
                self.fit_to_content.push(id);
                UPx::ZERO
            }
            GridDimension::Fractional { weight } => {
                self.total_weights += u32::from(weight);
                self.fractional.push((id, weight));
                UPx::ZERO
            }
            GridDimension::Measured { size: min, .. } => {
                self.premeasured.push(id);
                match min {
                    Dimension::Px(size) => self.allocated_space.0 += size.into_unsigned(),
                    Dimension::Lp(size) => self.allocated_space.1 += size,
                }
                min.into_upx(scale)
            }
        };
        self.layouts.insert(
            index,
            StackLayout {
                offset: UPx::ZERO,
                size: layout,
                baselines: Vec::new(),
            },
        );
    }

    #[allow(clippy::too_many_lines)] // TODO
    pub fn update(
        &mut self,
        available: Size<ConstraintLimit>,
        gutter: UPx,
        scale: Fraction,
        mut measure: impl FnMut(usize, usize, Size<ConstraintLimit>, bool) -> WidgetLayout,
    ) -> WidgetLayout {
        self.update_measured(scale);
        let (space_constraint, mut other_constraint) = self.orientation.split_size(available);
        let available_space = space_constraint.max();
        let known_gutters = gutter.saturating_mul(UPx::new(
            (self.children.len() - self.fit_to_content.len())
                .saturating_sub(1)
                .cast::<u32>(),
        ));
        let allocated_space =
            self.allocated_space.0 + self.allocated_space.1.into_upx(scale).ceil() + known_gutters;
        let mut remaining = available_space.saturating_sub(allocated_space);

        if self.elements_per_child > 1 {
            // When we are in multi-row mode, we force a size-to-fit mode for
            // children. Trying to ask each row to fill will never work.
            other_constraint = ConstraintLimit::SizeToFit(other_constraint.max());
        }

        // If our `other_constraint` is not known, we will need to give child
        // widgets an opportunity to lay themselves out in the full area. This
        // requires one extra layout call, so we avoid persisting layouts during
        // the first loop if this is the case.
        let needs_final_layout = !matches!(other_constraint, ConstraintLimit::Fill(_));

        // Measure the children that fit their content
        for other in &mut self.others {
            *other = UPx::ZERO;
        }
        let mut requires_gutter = false;
        for &id in &self.fit_to_content {
            let index = self.children.index_of_id(id).expect("child not found");

            let mut max_measured = UPx::ZERO;

            self.layouts[index].reset_baselines(self.elements_per_child);

            for element in 0..self.elements_per_child {
                let layout = measure(
                    index,
                    element,
                    self.orientation.make_size(
                        ConstraintLimit::SizeToFit(remaining.saturating_sub(if requires_gutter {
                            gutter
                        } else {
                            UPx::ZERO
                        })),
                        other_constraint,
                    ),
                    !needs_final_layout,
                );
                self.layouts[index].baselines[element] = layout.baseline;
                let (measured, other) = self.orientation.split_size(layout.size);

                if measured > 0 {
                    max_measured = max_measured.max(measured);
                    self.others[element] = self.others[element].max(other);
                }
            }
            self.layouts[index].size = max_measured;
            if max_measured > 0 {
                if requires_gutter {
                    remaining = remaining.saturating_sub(gutter);
                } else {
                    requires_gutter = true;
                }
            }
            remaining = remaining.saturating_sub(max_measured);
        }

        // Measure measure the "other" dimension for children that we know their size already.
        for &id in &self.premeasured {
            let index = self.children.index_of_id(id).expect("child not found");
            self.layouts[index].reset_baselines(self.elements_per_child);
            for element in 0..self.elements_per_child {
                let layout = measure(
                    index,
                    element,
                    self.orientation.make_size(
                        ConstraintLimit::Fill(self.layouts[index].size),
                        other_constraint,
                    ),
                    !needs_final_layout,
                );
                self.layouts[index].baselines[element] = layout.baseline;
                let (_, other) = self.orientation.split_size(layout.size);
                self.others[element] = self.others[element].max(other);
            }
        }

        // Measure the weighted children within the remaining space
        if self.total_weights > 0 {
            if requires_gutter {
                remaining = remaining.saturating_sub(gutter);
            }
            let space_per_weight = (remaining / self.total_weights).floor();
            remaining = remaining.saturating_sub(space_per_weight * self.total_weights);
            for (fractional_index, &(id, weight)) in self.fractional.iter().enumerate() {
                let index = self.children.index_of_id(id).expect("child not found");
                let mut size = space_per_weight * u32::from(weight);

                // If we have fractional amounts remaining, divide the pixels
                if remaining > 0 {
                    let from_end = u32::try_from(self.fractional.len() - fractional_index)
                        .expect("too many items");
                    if remaining >= from_end {
                        let amount = (remaining / from_end).ceil().min(remaining);
                        remaining -= amount;
                        size += amount;
                    }
                }

                self.layouts[index].size = size;
            }

            // Now that we know the constrained sizes, we can measure the children
            // to get the other measurement using the constrainted measurement.
            for (id, _) in &self.fractional {
                let index = self.children.index_of_id(*id).expect("child not found");
                self.layouts[index].reset_baselines(self.elements_per_child);
                for element in 0..self.elements_per_child {
                    let layout = measure(
                        index,
                        element,
                        self.orientation.make_size(
                            ConstraintLimit::Fill(self.layouts[index].size.into_upx(scale)),
                            other_constraint,
                        ),
                        !needs_final_layout,
                    );
                    let (_, measured) = self.orientation.split_size(layout.size);
                    self.others[element] = self.others[element].max(measured);
                }
            }
        }

        let mut total_other = self.total_other();
        if let ConstraintLimit::Fill(max) = other_constraint {
            let remaining = max.saturating_sub(total_other);
            if remaining > 0 {
                let other_count = self.others.len().cast::<u32>();
                let amount_per = (remaining / other_count).floor();
                let rounding_error = remaining - amount_per * other_count;
                self.others[0] += amount_per + rounding_error;
                for other in &mut self.others[1..] {
                    *other += amount_per;
                }
                total_other = max;
            }
        }

        let (measured, baseline) = self.update_offsets(needs_final_layout, gutter, scale, measure);

        WidgetLayout {
            size: self.orientation.make_size(measured, total_other),
            baseline,
        }
    }

    fn update_measured(&mut self, scale: Fraction) {
        if self.measured_scale != scale {
            self.measured_scale = scale;

            for (spec, layout) in self.children.iter().zip(self.layouts.iter_mut()) {
                let GridDimension::Measured { size } = spec else {
                    continue;
                };

                layout.size = size.into_upx(scale);
            }
        }
    }

    fn total_other(&self) -> UPx {
        self.others
            .iter()
            .fold(UPx::ZERO, |total, other| total.saturating_add(*other))
    }

    fn update_offsets(
        &mut self,
        needs_final_layout: bool,
        gutter: UPx,
        scale: Fraction,
        mut measure: impl FnMut(usize, usize, Size<ConstraintLimit>, bool) -> WidgetLayout,
    ) -> (UPx, Baseline) {
        let mut offset = UPx::ZERO;
        let first_baseline = match self.orientation {
            Orientation::Column => self.layouts.iter().fold(Baseline::NONE, |max, layout| {
                let baseline = layout.baselines.first().copied().unwrap_or_default();
                baseline.max(max)
            }),
            Orientation::Row => self
                .layouts
                .first()
                .and_then(|layout| layout.baselines.first().copied())
                .unwrap_or_default(),
        };
        for index in 0..self.children.len() {
            let visible = self.layouts[index].size > 0;

            if visible && offset > 0 {
                offset += gutter;
            }

            self.layouts[index].offset = offset;

            if visible {
                offset += self.layouts[index].size;
                if needs_final_layout {
                    for element in 0..self.elements_per_child {
                        measure(
                            index,
                            element,
                            self.orientation.make_size(
                                ConstraintLimit::Fill(self.layouts[index].size.into_upx(scale)),
                                ConstraintLimit::Fill(self.others[element]),
                            ),
                            true,
                        );
                    }
                }
            }
        }
        (offset, first_baseline)
    }
}

impl Deref for GridLayout {
    type Target = [StackLayout];

    fn deref(&self) -> &Self::Target {
        &self.layouts
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use figures::units::UPx;
    use figures::{Fraction, IntoSigned, Size, Zero};

    use super::{GridDimension, GridLayout, Orientation};
    use crate::styles::Dimension;
    use crate::ConstraintLimit;

    struct Child {
        size: UPx,
        dimension: GridDimension,
        other: UPx,
        divisible_by: Option<UPx>,
    }

    impl Child {
        pub fn new(size: impl Into<UPx>, other: impl Into<UPx>) -> Self {
            Self {
                size: size.into(),
                dimension: GridDimension::FitContent,
                other: other.into(),
                divisible_by: None,
            }
        }

        pub fn fixed_size(mut self, size: UPx) -> Self {
            self.dimension = GridDimension::Measured {
                size: Dimension::Px(size.into_signed()),
            };
            self
        }

        pub fn weighted(mut self, weight: u8) -> Self {
            self.dimension = GridDimension::Fractional { weight };
            self
        }

        pub fn divisible_by(mut self, split_at: impl Into<UPx>) -> Self {
            self.divisible_by = Some(split_at.into());
            self
        }
    }

    fn assert_measured_children_in_orientation(
        orientation: Orientation,
        children: &[Child],
        available: Size<ConstraintLimit>,
        expected: &[UPx],
        expected_size: Size<UPx>,
    ) {
        assert_eq!(children.len(), expected.len());
        let mut flex = GridLayout::new(orientation);
        for child in children {
            flex.push(child.dimension, Fraction::ONE);
        }

        let computed_layout = flex.update(
            available,
            UPx::ZERO,
            Fraction::ONE,
            |index, _element, constraints, _persist| {
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
                orientation.make_size(measured, other).into()
            },
        );
        assert_eq!(computed_layout.size, expected_size);
        let mut offset = UPx::ZERO;
        for ((index, child), &expected) in flex.iter().enumerate().zip(expected) {
            assert_eq!(
                child.size, expected,
                "child {index} measured to {}, expected {expected}",
                child.size,
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
            Orientation::Row,
            children,
            Orientation::Row.make_size(main_constraint, other_constraint),
            expected,
            Orientation::Row.make_size(expected_measured, expected_other),
        );
        assert_measured_children_in_orientation(
            Orientation::Column,
            children,
            Orientation::Column.make_size(main_constraint, other_constraint),
            expected,
            Orientation::Column.make_size(expected_measured, expected_other),
        );
    }

    #[test]
    fn size_to_fit() {
        assert_measured_children(
            &[Child::new(3, 1), Child::new(3, 1), Child::new(3, 1)],
            ConstraintLimit::SizeToFit(UPx::new(10)),
            ConstraintLimit::SizeToFit(UPx::new(10)),
            &[UPx::new(3), UPx::new(3), UPx::new(3)],
            UPx::new(9),
            UPx::new(1),
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
            ConstraintLimit::Fill(UPx::new(10)),
            ConstraintLimit::SizeToFit(UPx::new(10)),
            &[UPx::new(4), UPx::new(3), UPx::new(3)],
            UPx::new(10),
            UPx::new(7), // 20 / 3 = 6.666, rounded up is 7
        );
        // Same as above, but with an 11px box. This creates a leftover of 3 px
        // (11 % 4), adding 1px to all three children.
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Fill(UPx::new(11)),
            ConstraintLimit::SizeToFit(UPx::new(11)),
            &[UPx::new(5), UPx::new(3), UPx::new(3)],
            UPx::new(11),
            UPx::new(7), // 20 / 3 = 6.666, rounded up is 7
        );
        // 12px box. This creates no leftover.
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Fill(UPx::new(12)),
            ConstraintLimit::SizeToFit(UPx::new(12)),
            &[UPx::new(6), UPx::new(3), UPx::new(3)],
            UPx::new(12),
            UPx::new(4), // 20 / 6 = 3.666, rounded up is 4
        );
        // 13px box. This creates a leftover of 1 px (13 % 4), adding 1px only
        // to the final child
        assert_measured_children(
            &[
                Child::new(20, 1).divisible_by(3).weighted(2),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Fill(UPx::new(13)),
            ConstraintLimit::SizeToFit(UPx::new(13)),
            &[UPx::new(6), UPx::new(3), UPx::new(4)],
            UPx::new(13),
            UPx::new(4), // 20 / 6 = 3.666, rounded up is 4
        );
    }

    #[test]
    fn fixed_size() {
        assert_measured_children(
            &[
                Child::new(3, 1).fixed_size(UPx::new(7)),
                Child::new(3, 1).weighted(1),
                Child::new(3, 1).weighted(1),
            ],
            ConstraintLimit::Fill(UPx::new(15)),
            ConstraintLimit::SizeToFit(UPx::new(15)),
            &[UPx::new(7), UPx::new(4), UPx::new(4)],
            UPx::new(15),
            UPx::new(1),
        );
    }
}

/// A 2d collection of widgets for a [`Grid`].
#[derive(Debug, Default, Eq, PartialEq)]
pub struct GridWidgets<const N: usize>(Vec<GridSection<N>>);

impl<const N: usize> GridWidgets<N> {
    /// Returns an empty collection of widgets.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Pushes another `section` of widgets and returns the updated collection.
    #[must_use]
    pub fn and(mut self, section: impl Into<GridSection<N>>) -> Self {
        self.push(section.into());
        self
    }
}

impl<T, const N: usize> From<Vec<T>> for GridWidgets<N>
where
    T: Into<GridSection<N>>,
{
    fn from(value: Vec<T>) -> Self {
        Self(value.into_iter().map(T::into).collect())
    }
}

impl<T, const N: usize> From<T> for GridWidgets<N>
where
    T: Into<GridSection<N>>,
{
    fn from(value: T) -> Self {
        Self(vec![value.into()])
    }
}

impl<A, const N: usize> FromIterator<A> for GridWidgets<N>
where
    A: Into<GridSection<N>>,
{
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self(iter.into_iter().map(A::into).collect())
    }
}

impl<const N: usize> Deref for GridWidgets<N> {
    type Target = Vec<GridSection<N>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for GridWidgets<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A single dimension of widgets within a [`GridWidgets`] collection.
#[derive(Debug, Eq, PartialEq)]
pub struct GridSection<const N: usize>([WidgetInstance; N]);

impl Default for GridSection<0> {
    fn default() -> Self {
        Self::new()
    }
}

impl GridSection<0> {
    /// Returns an empty section.
    #[must_use]
    pub const fn new() -> Self {
        Self([])
    }

    /// Appends `other` to the end of this collection of widgets and
    /// returns the updated collection.
    #[must_use]
    pub fn and(self, other: impl MakeWidget) -> GridSection<1> {
        GridSection([other.make_widget()])
    }
}

impl<T> From<T> for GridSection<1>
where
    T: MakeWidget,
{
    fn from(value: T) -> Self {
        Self([value.make_widget()])
    }
}

impl<const N: usize, T> From<[T; N]> for GridSection<N>
where
    T: MakeWidget,
{
    fn from(values: [T; N]) -> Self {
        let mut widgets = values.into_iter();
        Self(array::from_fn(|_| {
            widgets.next().assert("length checked").make_widget()
        }))
    }
}

impl<const N: usize> Deref for GridSection<N> {
    type Target = [WidgetInstance; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for GridSection<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! impl_grid_widgets_and {
    ($($var:ident $num:literal)+) => {
        impl_grid_widgets_and!([] $($var $num)+ );
    };
    ([$($done:ident $done_num:literal)*] $cur:ident $cur_num:literal ) => {};
    ([$($done:ident $done_num:literal)*] $cur:ident $cur_num:literal $next:ident $next_num:literal $($var:ident $num:literal)* ) => {
        impl GridSection<$cur_num> {
            /// Appends `other` to the end of this collection of widgets and
            /// returns the updated collection.
            #[must_use]
            pub fn and(self, other: impl MakeWidget) -> GridSection<$next_num> {
                let mut items = self.0.into_iter();
                $(
                    let $done = items.next().assert("known size");
                )*
                GridSection([
                    $($done,)*
                    items.next().assert("known size"),
                    other.make_widget()
                ])
            }
        }

        impl_grid_widgets_and!([$($done $done_num)* $cur $cur_num] $next $next_num $($var $num)* );
    };
}

impl_grid_widgets_and!(a1 1 a2 2 a3 3 a4 4 a5 5 a6 6 a7 7 a8 8 a9 9 a10 10 a11 11 a12 12);

macro_rules! impl_grid_widgets_from_tuple {
    ($($type:ident $field:tt $var:ident),+) => {
        impl<$($type),+> From<($($type,)+)> for GridSection<{ $crate::count!($($field),+;) }>
        where
            $($type: MakeWidget,)+
        {
            fn from(tuple: ($($type,)+)) -> Self {
                Self([
                    $(tuple.$field.make_widget(),)+
                ])
            }
        }
    };
}

impl_all_tuples!(impl_grid_widgets_from_tuple);
