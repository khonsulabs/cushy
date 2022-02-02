use std::ops::Deref;

use gooey_core::{
    figures::{Figure, Point, Rectlike, Size, SizedRect, Vector, Vectorlike},
    Scaled, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{ContentArea, Rasterizer, Renderer, WidgetRasterizer};

use super::{LayoutChild, WidgetLayout};
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
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        let bounds = content_area.bounds();
        for_each_measured_widget(context, bounds.size(), |layout, child_bounds| {
            context.frontend.with_transmogrifier(
                layout.registration.id(),
                |transmogrifier, mut child_context| {
                    transmogrifier.render_within(
                        &mut child_context,
                        child_bounds.translate(content_area.location).as_rect(),
                        Some(context.registration.id()),
                        context.style(),
                    );
                },
            );
        });
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        let mut extents = Vector::default();
        let context_size = context.frontend.renderer().unwrap().size();
        let constrained_size = Size::new(
            constraints.width.unwrap_or(context_size.width),
            constraints.height.unwrap_or(context_size.height),
        );
        for_each_measured_widget(context, constrained_size, |_layout, child_bounds| {
            extents = extents.max(&child_bounds.as_extents().extent.to_vector());
        });
        extents.to_size()
    }
}

#[allow(clippy::cast_precision_loss)]
fn for_each_measured_widget<R: Renderer, F: FnMut(&LayoutChild, SizedRect<f32, Scaled>)>(
    context: &TransmogrifierContext<'_, LayoutTransmogrifier, Rasterizer<R>>,
    constraints: Size<f32, Scaled>,
    callback: F,
) {
    for_each_widget(
        context.widget.children.layout_children(),
        constraints,
        |child| {
            let layout_surround = child.layout.surround_in_points(constraints);
            let layout_max_size = child
                .layout
                .size_in_points(constraints)
                .min(&(constraints - layout_surround.minimum_size()));
            // Constrain the child to whatever remains after the left/right/top/bottom
            // measurements
            let child_constraints =
                Size::new(Some(layout_max_size.width), Some(layout_max_size.height));
            context
                .frontend
                .with_transmogrifier(
                    child.registration.id(),
                    |transmogrifier, mut child_context| {
                        transmogrifier
                            .content_size(&mut child_context, child_constraints)
                            .total_size()
                    },
                )
                .unwrap_or_default()
        },
        callback,
    );
}

#[allow(clippy::cast_precision_loss)]
fn for_each_widget<
    C: Deref<Target = WidgetLayout>,
    F: FnMut(&C, SizedRect<f32, Scaled>),
    W: Fn(&C) -> Size<f32, Scaled>,
>(
    // context: &TransmogrifierContext<'_, LayoutTransmogrifier, Rasterizer<R>>,
    children: Vec<C>,
    constraints: Size<f32, Scaled>,
    child_measurer: W,
    mut callback: F,
) {
    for child in children {
        let layout_surround = child.surround_in_points(constraints);

        // Ask the child to measure
        let child_size = child_measurer(&child);

        // If the layout has an explicit width/height, we should return it if it's a
        // value larger than what the child reported
        let child_size = Size::from_figures(
            child
                .width
                .length(constraints.width())
                .unwrap_or_else(|| child_size.width()),
            child
                .height
                .length(constraints.height())
                .unwrap_or_else(|| child_size.height()),
        );
        let remaining_size = constraints - child_size;
        let (left, width) = calculate_origin_length(
            layout_surround.left,
            layout_surround.right,
            child_size.width(),
            remaining_size.width(),
        );
        let (top, height) = calculate_origin_length(
            layout_surround.top,
            layout_surround.bottom,
            child_size.height(),
            remaining_size.height(),
        );
        callback(
            &child,
            SizedRect::new(
                Point::from_figures(left, top),
                Size::from_figures(width, height).max(&Size::default()),
            ),
        );
    }
}

fn calculate_origin_length(
    origin: Option<Figure<f32, Scaled>>,
    extent: Option<Figure<f32, Scaled>>,
    measured: Figure<f32, Scaled>,
    remaining: Figure<f32, Scaled>,
) -> (Figure<f32, Scaled>, Figure<f32, Scaled>) {
    match (origin, extent) {
        (Some(origin), Some(extent)) => {
            // Length is calculated
            (origin, measured + remaining - extent - origin)
        }
        (Some(top), None) => {
            // Extent would be calculated, but doesn't need to be
            (top, measured)
        }
        (None, Some(bottom)) => {
            // Extent is calculated
            (remaining - bottom, measured)
        }
        (None, None) => (Figure::default(), measured),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use gooey_core::figures::{Approx, Point, Scaled, Size, SizedRect};

    use crate::layout::{rasterizer::for_each_widget, Dimension, WidgetLayout};

    struct TestCase {
        layout: WidgetLayout,
        result: SizedRect<f32, Scaled>,
    }

    impl Deref for TestCase {
        type Target = WidgetLayout;

        fn deref(&self) -> &WidgetLayout {
            &self.layout
        }
    }

    #[test]
    #[cfg(feature = "frontend-rasterizer")]
    fn layout_tests() {
        // Each test case will have a widget that returns a measurement of 30x20
        // The constraints will be 100,90.
        let widget_size = Size::new(30., 20.);
        let cases = vec![
            TestCase {
                layout: WidgetLayout::default(),
                result: SizedRect::new(Point::default(), widget_size),
            },
            TestCase {
                layout: WidgetLayout::build()
                    .left(Dimension::exact(10.))
                    .top(Dimension::exact(20.))
                    .finish(),
                result: SizedRect::new(Point::new(10., 20.), widget_size),
            },
            TestCase {
                layout: WidgetLayout::build()
                    .right(Dimension::exact(10.))
                    .bottom(Dimension::exact(20.))
                    .finish(),
                result: SizedRect::new(Point::new(60., 50.), widget_size),
            },
            TestCase {
                layout: WidgetLayout::build()
                    .left(Dimension::exact(10.))
                    .top(Dimension::exact(20.))
                    .width(Dimension::exact(40.))
                    .height(Dimension::exact(30.))
                    .finish(),
                result: SizedRect::new(Point::new(10., 20.), Size::new(40., 30.)),
            },
            TestCase {
                layout: WidgetLayout::build()
                    .left(Dimension::exact(10.))
                    .top(Dimension::exact(20.))
                    .right(Dimension::exact(30.))
                    .bottom(Dimension::exact(40.))
                    .finish(),
                result: SizedRect::new(Point::new(10., 20.), Size::new(60., 30.)),
            },
            TestCase {
                layout: WidgetLayout::build()
                    .width(Dimension::exact(40.))
                    .height(Dimension::exact(30.))
                    .right(Dimension::exact(30.))
                    .bottom(Dimension::exact(40.))
                    .finish(),
                result: SizedRect::new(Point::new(30., 20.), Size::new(40., 30.)),
            },
        ];

        for_each_widget(
            cases,
            Size::new(100., 90.),
            |_| widget_size,
            |case, result| {
                assert!(
                    case.result.approx_eq(&result),
                    "Layout {:?} produced {:?}, expected {:?}",
                    case.layout,
                    result,
                    case.result
                );
            },
        );
    }
}
