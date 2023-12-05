use std::fmt;

use gooey::widget::{RootBehavior, WidgetInstance};
use kludgine::figures::units::UPx;
use kludgine::figures::{IntoSigned, Rect, Size, Zero};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext};
use crate::value::{Generation, IntoValue, Value};
use crate::widget::{Children, ManagedWidget, Widget};
use crate::ConstraintLimit;

/// A Z-direction stack of widgets.
#[derive(Debug)]
pub struct Layers {
    /// The children that are laid out as layers with index 0 being the lowest (bottom).
    pub children: Value<Children>,
    mounted: Vec<ManagedWidget>,
    mounted_generation: Option<Generation>,
}

impl Layers {
    /// Returns a new instance that lays out `children` as layers.
    pub fn new(children: impl IntoValue<Children>) -> Self {
        Self {
            children: children.into_value(),
            mounted: Vec::new(),
            mounted_generation: None,
        }
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_, '_>) {
        let current_generation = self.children.generation();
        self.children.invalidate_when_changed(context);
        if current_generation.map_or_else(
            || self.children.map(Children::len) != self.mounted.len(),
            |gen| Some(gen) != self.mounted_generation,
        ) {
            self.mounted_generation = self.children.generation();
            self.children.map(|children| {
                for (index, widget) in children.iter().enumerate() {
                    if self
                        .mounted
                        .get(index)
                        .map_or(true, |child| child != widget)
                    {
                        // These entries do not match. See if we can find the
                        // new id somewhere else, if so we can swap the entries.
                        if let Some((swap_index, _)) = self
                            .mounted
                            .iter()
                            .enumerate()
                            .skip(index + 1)
                            .find(|(_, child)| *child == widget)
                        {
                            self.mounted.swap(index, swap_index);
                        } else {
                            // This is a brand new child.
                            self.mounted
                                .insert(index, context.push_child(widget.clone()));
                        }
                    }
                }

                // Any children remaining at the end of this process are ones
                // that have been removed.
                for removed in self.mounted.drain(children.len()..) {
                    context.remove_child(&removed);
                }
            });
        }
    }
}

impl Widget for Layers {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.synchronize_children(&mut context.as_event_context());

        for child in &self.mounted {
            context.for_other(child).redraw();
        }
    }

    fn summarize(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.children.map(|children| {
            let mut f = f.debug_tuple("Layered");
            for child in children {
                f.field(child);
            }

            f.finish()
        })
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        self.synchronize_children(&mut context.as_event_context());

        let mut size = Size::ZERO;

        for child in &self.mounted {
            size = size.max(
                context
                    .for_other(child)
                    .as_temporary()
                    .layout(available_space),
            );
        }

        // Now we know the size of the widget, we can request the widgets fill
        // the allocated space.
        let layout = Rect::from(size).into_signed();
        for child in &self.mounted {
            context
                .for_other(child)
                .layout(size.map(ConstraintLimit::Fill));
            context.set_child_layout(child, layout);
        }

        size
    }

    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {
        self.synchronize_children(context);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {
        for child in self.mounted.drain(..) {
            context.remove_child(&child);
        }
        self.mounted_generation = None;
    }

    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_, '_>,
    ) -> Option<(RootBehavior, WidgetInstance)> {
        self.synchronize_children(context);

        for child in &self.mounted {
            let Some((child_behavior, next_in_chain)) = context.for_other(child).root_behavior()
            else {
                continue;
            };

            return Some((child_behavior, next_in_chain));
        }

        None
    }
}
