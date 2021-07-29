use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D, Vector2D},
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, Renderer, WidgetRasterizer};

use super::LayoutChild;
use crate::layout::{Layout, LayoutTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for LayoutTransmogrifier {
    type State = ();
    type Widget = Layout;

    fn receive_command(
        &self,
        _command: <Self::Widget as gooey_core::Widget>::Command,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for LayoutTransmogrifier {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
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
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
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

#[allow(clippy::cast_precision_loss)]
fn for_each_measured_widget<R: Renderer, F: FnMut(&LayoutChild, Rect<f32, Points>)>(
    context: &TransmogrifierContext<'_, LayoutTransmogrifier, Rasterizer<R>>,
    constraints: Size2D<f32, Points>,
    mut callback: F,
) {
    for child in context.widget.children.layout_children() {
        let layout_surround = child.layout.surround_in_points(constraints);
        let minimum_surround = layout_surround.minimum_size();
        let child_constrained_size = constraints - minimum_surround;
        let layout_max_size = child.layout.size_in_points(child_constrained_size);
        // Constrain the child to whatever remains after the left/right/top/bottom
        // measurements
        let child_constraints =
            Size2D::new(Some(layout_max_size.width), Some(layout_max_size.height));
        // Ask the child to measure
        let child_size = context
            .frontend
            .with_transmogrifier(
                child.registration.id(),
                |transmogrifier, mut child_context| {
                    transmogrifier.content_size(&mut child_context, child_constraints)
                },
            )
            .expect("unknown transmogrifier");

        // If the layout has an explicit width/height, we should return it if it's a
        // value larger than what the child reported
        let child_size = child_size.max(layout_max_size);
        let remaining_size = constraints - child_size;
        // If either top or left are Auto, we look at bottom or right,
        // respectively. If the corresponding dimension is also Auto, we divide
        // the remaining amount in two. If the other dimension is specified,
        // however, we subtract its measurement from the remaining value and use
        // it as top/left.
        let child_left = layout_surround.left.unwrap_or_else(|| {
            let remaining_width = Length::new(remaining_size.width);
            layout_surround
                .right
                .map_or_else(|| remaining_width / 2., |right| remaining_width - right)
        });
        let child_top = layout_surround.top.unwrap_or_else(|| {
            let remaining_height = Length::new(remaining_size.height);
            layout_surround
                .bottom
                .map_or_else(|| remaining_height / 2., |bottom| remaining_height - bottom)
        });
        callback(
            &child,
            Rect::new(Point2D::from_lengths(child_left, child_top), child_size),
        );
    }
}
