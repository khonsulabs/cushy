use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D, Vector2D},
    renderer::Renderer,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use super::LayoutChild;
use crate::layout::{Layout, LayoutTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for LayoutTransmogrifier {
    type State = ();
    type Widget = Layout;
}

impl<R: Renderer> WidgetRasterizer<R> for LayoutTransmogrifier {
    fn render(&self, context: TransmogrifierContext<Self, Rasterizer<R>>) {
        let context_size = context.frontend.renderer().unwrap().size();
        for_each_measured_widget(&context, context_size, |layout, child_bounds| {
            context.frontend.with_transmogrifier(
                layout.registration.id(),
                |transmogrifier, mut child_context| {
                    transmogrifier.render_within(&mut child_context, child_bounds, context.style);
                },
            );
        });
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        let mut extents = Vector2D::default();
        let context_size = context.frontend.renderer().unwrap().size();
        let constrained_size = Size2D::new(
            constraints.width.unwrap_or(context_size.width),
            constraints.height.unwrap_or(context_size.height),
        );
        for_each_measured_widget(&context, constrained_size, |_layout, child_bounds| {
            extents = extents.max(child_bounds.max().to_vector());
        });
        extents.to_size()
    }
}

fn for_each_measured_widget<R: Renderer, F: FnMut(&LayoutChild, Rect<f32, Points>)>(
    context: &TransmogrifierContext<LayoutTransmogrifier, Rasterizer<R>>,
    constraints: Size2D<f32, Points>,
    mut callback: F,
) {
    for child in context.widget.children.layout_children() {
        let layout_surround = child.layout.surround_in_points(&constraints);
        let child_constrained_size = Size2D::from_lengths(
            Length::new(constraints.width) - layout_surround.minimum_width(),
            Length::new(constraints.height) - layout_surround.minimum_height(),
        );
        // Constrain the child to whatever remains after the left/right/top/bottom
        // measurements
        let child_constraints = Size2D::new(
            Some(child_constrained_size.width),
            Some(child_constrained_size.height),
        );
        // Ask the child to measure
        let child_size = context
            .frontend
            .with_transmogrifier(
                child.registration.id(),
                |transmogrifier, mut child_context| {
                    transmogrifier.content_size(&mut child_context, child_constraints)
                },
            )
            .unwrap();

        // If the layout has an explicit width/height, we should return it if it's a
        // value larger than what the child reported
        let child_size = child_size.max(child.layout.size_in_points(&child_constrained_size));
        // If either top or left are Auto, we need to divide it equally with the
        // corresponding measurement if it's also auto.
        let child_left = layout_surround.left.unwrap_or_else(|| {
            Length::new(child_size.width)
                / count_autos(layout_surround.left, layout_surround.right) as f32
        });
        let child_top = layout_surround.top.unwrap_or_else(|| {
            Length::new(child_size.height)
                / count_autos(layout_surround.top, layout_surround.bottom) as f32
        });
        callback(
            &child,
            Rect::new(Point2D::from_lengths(child_left, child_top), child_size),
        );
    }
}

fn count_autos(a: Option<Length<f32, Points>>, b: Option<Length<f32, Points>>) -> usize {
    count_auto(a) + count_auto(b)
}

fn count_auto(a: Option<Length<f32, Points>>) -> usize {
    if a.is_none() {
        1
    } else {
        0
    }
}
