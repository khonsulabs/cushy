//! A widget that combines a collection of [`Children`] widgets into one.

use kludgine::figures::units::UPx;
use kludgine::figures::{IntoSigned, Rect, ScreenScale, Size};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::IntrinsicPadding;
use crate::value::{Generation, IntoValue, Value};
use crate::widget::{Children, ManagedWidget, Widget, WidgetRef};
use crate::widgets::grid::{GridDimension, GridLayout, Orientation};
use crate::widgets::{Expand, Resize};
use crate::ConstraintLimit;

/// A widget that displays a collection of [`Children`] widgets in a
/// [orientation](Orientation).
#[derive(Debug)]
pub struct Stack {
    orientation: Orientation,
    /// The children widgets that belong to this array.
    pub children: Value<Children>,
    layout: GridLayout,
    layout_generation: Option<Generation>,
    // TODO Refactor synced_children into its own type.
    synced_children: Vec<ManagedWidget>,
}

impl Stack {
    /// Returns a new widget with the given orientation and widgets.
    pub fn new(orientation: Orientation, widgets: impl IntoValue<Children>) -> Self {
        Self {
            orientation,
            children: widgets.into_value(),
            layout: GridLayout::new(orientation),
            layout_generation: None,
            synced_children: Vec::new(),
        }
    }

    /// Returns a new instance that displays `widgets` in a series of columns.
    pub fn columns(widgets: impl IntoValue<Children>) -> Self {
        Self::new(Orientation::Column, widgets)
    }

    /// Returns a new instance that displays `widgets` in a series of rows.
    pub fn rows(widgets: impl IntoValue<Children>) -> Self {
        Self::new(Orientation::Row, widgets)
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_, '_>) {
        let current_generation = self.children.generation();
        self.children.invalidate_when_changed(context);
        if current_generation.map_or_else(
            || self.children.map(Children::len) != self.layout.len(),
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
                            let (mut widget, dimension) = if let Some((weight, expand)) =
                                guard.downcast_ref::<Expand>().and_then(|expand| {
                                    expand
                                        .weight(self.orientation == Orientation::Row)
                                        .map(|weight| (weight, expand))
                                }) {
                                (expand.child().clone(), GridDimension::Fractional { weight })
                            } else if let Some((child, size)) =
                                guard.downcast_ref::<Resize>().and_then(|r| {
                                    let range = match self.layout.orientation {
                                        Orientation::Row => r.height,
                                        Orientation::Column => r.width,
                                    };
                                    range.minimum().map(|size| {
                                        (r.child().clone(), GridDimension::Measured { size })
                                    })
                                })
                            {
                                (child, size)
                            } else {
                                (
                                    WidgetRef::Unmounted(widget.clone()),
                                    GridDimension::FitContent,
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
            context.get(&IntrinsicPadding).into_upx(context.gfx.scale()),
            context.gfx.scale(),
            |child_index, _element, constraints, persist| {
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
                            .make_point(layout.offset, UPx::ZERO)
                            .into_signed(),
                        self.layout
                            .orientation
                            .make_size(layout.size, self.layout.others[0])
                            .into_signed(),
                    ),
                );
            }
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
