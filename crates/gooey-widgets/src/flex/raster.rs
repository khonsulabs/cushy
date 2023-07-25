mod layout;
use std::collections::HashSet;
use std::num::NonZeroUsize;

use alot::LotId;
use gooey_core::math::units::{Px, UPx};
use gooey_core::math::{IntoSigned, Point, Rect, Size};
use gooey_core::reactor::Dynamic;
use gooey_core::{Children, Value, WidgetTransmogrifier};
use gooey_raster::{
    AnyRasterContext, ConstraintLimit, RasterContext, Rasterizable, RasterizedApp, Renderer,
    WidgetRasterizer,
};

use crate::flex::{FlexDimension, FlexTransmogrifier};
use crate::Flex;

struct FlexRasterizer {
    children_source: Option<ChildrenSource>,
    children: RasterizedChildren,
    flex: layout::Flex,
    mouse_tracking: Option<LotId>,
    hovering: HashSet<LotId>,
}

struct ChildrenSource {
    children: Dynamic<Children>,
    generation: Option<NonZeroUsize>,
}

impl FlexRasterizer {
    fn synchronize_children(&mut self, context: &mut dyn AnyRasterContext) {
        let Some(source) = &mut self.children_source else { return };
        if source.generation != source.children.generation() {
            source.generation = source.children.generation();
            source.children.map_ref(|source| {
                for (index, (id, source)) in source.entries().enumerate() {
                    if self.children.get(index).map(|(id, _)| *id) != Some(id) {
                        // These entries do not match. See if we can find the
                        // new id somewhere else, if so we can swap the entries.
                        if let Some((swap_index, _)) = self
                            .children
                            .iter()
                            .enumerate()
                            .skip(index + 1)
                            .find(|(_, child)| child.0 == id)
                        {
                            self.children.swap(index, swap_index);
                            self.flex.swap(index, swap_index);
                        } else {
                            // This is a brand new child.
                            let rasterizable = context.instantiate(source);
                            self.children.insert(index, (id, rasterizable));
                            self.flex.insert(index, FlexDimension::FitContent);
                        }
                    }
                }

                // Any children remaining at the end of this process are ones
                // that have been removed.
                self.children.truncate(source.len());
                self.flex.truncate(source.len());
            });
        }
    }
}

type RasterizedChildren = Vec<(LotId, Rasterizable)>;

impl<Surface> WidgetTransmogrifier<RasterizedApp<Surface>> for FlexTransmogrifier
where
    Surface: gooey_raster::Surface,
{
    type Widget = Flex;

    fn transmogrify(
        &self,
        widget: &Self::Widget,
        style: gooey_core::reactor::Dynamic<stylecs::Style>,
        context: &RasterContext<Surface>,
    ) -> Rasterizable {
        let mut raster_children = RasterizedChildren::default();
        let mut flex = layout::Flex::new(widget.direction.get());
        widget.children.map_ref(|children| {
            for (id, child) in children.entries() {
                flex.push(FlexDimension::FitContent);
                raster_children.push((
                    id,
                    context
                        .widgets()
                        .instantiate(&*child.widget, style, context),
                ));
            }
        });

        if let Value::Dynamic(value) = &widget.direction {
            value.for_each({
                let handle = context.handle().clone();
                move |_| {
                    handle.invalidate();
                }
            })
        }

        let children_source = if let Value::Dynamic(value) = widget.children {
            value.for_each({
                let handle = context.handle().clone();
                move |_| {
                    handle.invalidate();
                }
            });
            Some(ChildrenSource {
                children: value,
                // TODO this generation call should be before we get the children...
                generation: value.generation(),
            })
        } else {
            None
        };

        Rasterizable::new(FlexRasterizer {
            children_source,
            children: raster_children,
            flex,
            mouse_tracking: None,
            hovering: HashSet::new(),
        })
    }
}

impl WidgetRasterizer for FlexRasterizer {
    type Widget = Flex;

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        renderer: &mut dyn Renderer,
        context: &mut dyn AnyRasterContext,
    ) -> Size<UPx> {
        self.synchronize_children(context);
        self.flex
            .update(available_space, |child_index, constraints| {
                self.children[child_index]
                    .1
                    .measure(constraints, renderer, context)
            })
    }

    fn draw(&mut self, renderer: &mut dyn Renderer, context: &mut dyn AnyRasterContext) {
        self.synchronize_children(context);
        self.flex.update(
            Size::new(
                ConstraintLimit::Known(renderer.size().width),
                ConstraintLimit::Known(renderer.size().height),
            ),
            |child_index, constraints| {
                self.children[child_index]
                    .1
                    .measure(constraints, renderer, context)
            },
        );

        for (layout, (_id, rasterizable)) in self.flex.iter().zip(self.children.iter_mut()) {
            renderer.clip_to(Rect::new(
                self.flex.orientation.make_point(layout.offset, UPx(0)),
                self.flex
                    .orientation
                    .make_size(layout.size, self.flex.other),
            ));
            rasterizable.draw(renderer, context);
            renderer.pop_clip();
        }
    }

    fn mouse_down(&mut self, location: Point<Px>, context: &mut dyn AnyRasterContext) {
        for (layout, (id, rasterizable)) in self.flex.iter().zip(self.children.iter_mut()) {
            let rect = Rect::new(
                self.flex.orientation.make_point(layout.offset, UPx(0)),
                self.flex
                    .orientation
                    .make_size(layout.size, self.flex.other),
            )
            .into_signed();
            let relative = location - rect.origin;
            if relative.x >= 0
                && relative.y >= 0
                && relative.x < rect.size.width
                && relative.y < rect.size.height
            {
                self.mouse_tracking = Some(*id);
                rasterizable.mouse_down(relative, context);
                break;
            }
        }
    }

    fn cursor_moved(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {
        for (layout, (id, rasterizable)) in self.flex.iter().zip(self.children.iter_mut()) {
            let rect = Rect::new(
                self.flex.orientation.make_point(layout.offset, UPx(0)),
                self.flex
                    .orientation
                    .make_size(layout.size, self.flex.other),
            )
            .into_signed();
            let relative = location.map(|location| location - rect.origin);
            if relative.map_or(false, |relative| {
                relative.x >= 0
                    && relative.y >= 0
                    && relative.x < rect.size.width
                    && relative.y < rect.size.height
            }) {
                rasterizable.cursor_moved(relative, context);
                self.hovering.insert(*id);
            } else if self.hovering.remove(id) {
                rasterizable.cursor_moved(None, context);
            }
        }
    }

    fn mouse_up(&mut self, location: Option<Point<Px>>, context: &mut dyn AnyRasterContext) {
        if let Some((layout, (_, rasterizable))) = self
            .flex
            .iter()
            .zip(self.children.iter_mut())
            .find(|(_, (id, _))| Some(*id) == self.mouse_tracking)
        {
            let rect = Rect::new(
                self.flex.orientation.make_point(layout.offset, UPx(0)),
                self.flex
                    .orientation
                    .make_size(layout.size, self.flex.other),
            )
            .into_signed();
            let relative = location.map(|location| location - rect.origin);
            if relative.map_or(false, |relative| {
                relative.x >= 0
                    && relative.y >= 0
                    && relative.x < rect.size.width
                    && relative.y < rect.size.height
            }) {
                rasterizable.mouse_up(relative, context);
            } else {
                rasterizable.mouse_up(None, context);
            }
        }
        self.mouse_tracking = None;
    }
}
