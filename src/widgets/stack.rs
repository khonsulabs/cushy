//! A widget that combines a collection of [`WidgetList`] widgets into one.

use figures::units::UPx;
use figures::{Round, ScreenScale, Size, Zero};

use super::expand::ExpandKind;
use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, Trackable};
use crate::reactive::value::{Generation, IntoValue, Value};
use crate::styles::components::{IntrinsicPadding, VerticalAlignment};
use crate::styles::FlexibleDimension;
use crate::widget::{
    ChildrenSyncChange, MountedWidget, Widget, WidgetLayout, WidgetList, WidgetRef,
};
use crate::widgets::grid::{GridDimension, GridLayout, Orientation};
use crate::widgets::{Expand, Resize};
use crate::ConstraintLimit;

/// A widget that displays a collection of [`WidgetList`] widgets in a
/// [orientation](Orientation).
#[derive(Debug)]
pub struct Stack {
    orientation: Orientation,
    /// The children widgets that belong to this array.
    pub children: Value<WidgetList>,
    /// The amount of space to place between each widget.
    pub gutter: Value<FlexibleDimension>,
    layout: GridLayout,
    layout_generation: Option<Generation>,
    synced_children: Vec<MountedWidget>,
}

impl Stack {
    /// Returns a new widget with the given orientation and widgets.
    pub fn new(orientation: Orientation, widgets: impl IntoValue<WidgetList>) -> Self {
        Self {
            orientation,
            children: widgets.into_value(),
            gutter: Value::Constant(FlexibleDimension::Auto),
            layout: GridLayout::new(orientation),
            layout_generation: None,
            synced_children: Vec::new(),
        }
    }

    /// Returns a new instance that displays `widgets` in a series of columns.
    pub fn columns(widgets: impl IntoValue<WidgetList>) -> Self {
        Self::new(Orientation::Column, widgets)
    }

    /// Returns a new instance that displays `widgets` in a series of rows.
    pub fn rows(widgets: impl IntoValue<WidgetList>) -> Self {
        Self::new(Orientation::Row, widgets)
    }

    /// Sets the space between each child to `gutter` and returns self.
    #[must_use]
    pub fn gutter(mut self, gutter: impl IntoValue<FlexibleDimension>) -> Self {
        self.gutter = gutter.into_value();
        self
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_>) {
        let current_generation = self.children.generation();
        self.children.invalidate_when_changed(context);
        if current_generation.map_or_else(
            || self.children.map(WidgetList::len) != self.layout.len(),
            |gen| Some(gen) != self.layout_generation,
        ) {
            self.layout_generation = self.children.generation();
            self.children.map(|children| {
                children.synchronize_with(
                    &mut self.synced_children,
                    |this, index| this.get(index).map(MountedWidget::instance),
                    |this, change| match change {
                        ChildrenSyncChange::Insert(index, widget) => {
                            // This is a brand new child.
                            let mut guard = widget.lock();
                            let (mut widget, dimension) = if let Some((weight, expand)) =
                                guard.downcast_mut::<Expand>().and_then(|expand| {
                                    expand
                                        .weight(self.orientation == Orientation::Row)
                                        .map(|weight| (weight, expand))
                                }) {
                                let widget = match expand.expand_kind() {
                                    ExpandKind::Horizontal
                                        if self.orientation == Orientation::Row =>
                                    {
                                        WidgetRef::new(widget.clone())
                                    }
                                    ExpandKind::Vertical
                                        if self.orientation == Orientation::Column =>
                                    {
                                        WidgetRef::new(widget.clone())
                                    }
                                    _ => expand.child().clone(),
                                };
                                (widget, GridDimension::Fractional { weight })
                            } else if let Some((child, size)) =
                                guard.downcast_ref::<Resize>().and_then(|r| {
                                    let (range, other_range) = match self.layout.orientation {
                                        Orientation::Row => (r.height, r.width),
                                        Orientation::Column => (r.width, r.height),
                                    };
                                    let cell = if other_range.is_unbounded() {
                                        r.child().clone()
                                    } else {
                                        WidgetRef::new(widget.clone())
                                    };
                                    range
                                        .minimum()
                                        .map(|size| (cell, GridDimension::Measured { size }))
                                })
                            {
                                (child, size)
                            } else {
                                (WidgetRef::new(widget.clone()), GridDimension::FitContent)
                            };
                            drop(guard);
                            this.insert(index, widget.mounted(context));

                            self.layout
                                .insert(index, dimension, context.kludgine.scale());
                        }
                        ChildrenSyncChange::Swap(a, b) => {
                            this.swap(a, b);
                            self.layout.swap(a, b);
                        }
                        ChildrenSyncChange::Truncate(length) => {
                            for removed in this.drain(length..) {
                                context.remove_child(&removed);
                            }
                            self.layout.truncate(length);
                        }
                    },
                );
            });
        }
    }
}

impl Widget for Stack {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        for child in &self.synced_children {
            context.for_other(child).redraw();
        }
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        for child in &mut self.synced_children {
            child.remount_if_needed(context);
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        self.synchronize_children(&mut context.as_event_context());
        let cell_align = context.get(&VerticalAlignment);

        self.gutter.invalidate_when_changed(context);
        let gutter = match self.gutter.get() {
            FlexibleDimension::Auto => context.get(&IntrinsicPadding),
            FlexibleDimension::Dimension(dimension) => dimension,
        }
        .into_upx(context.gfx.scale())
        .round();

        let content_size = self.layout.update(
            available_space,
            gutter,
            context.gfx.scale(),
            |child_index, _element, constraints, persist| {
                let mut context = context.for_other(&self.synced_children[child_index]);
                if !persist {
                    context = context.as_temporary();
                }
                context.layout(constraints)
            },
        );

        for (index, (layout, child)) in self.layout.iter().zip(&self.synced_children).enumerate() {
            let position = self.layout.position_cell(
                layout,
                UPx::ZERO,
                self.layout.others[0],
                index,
                0,
                cell_align,
            );

            context.set_child_layout(child, position);
        }

        content_size
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Stack")
            .field("orientation", &self.layout.orientation)
            .field("children", &self.children)
            .finish()
    }
}
