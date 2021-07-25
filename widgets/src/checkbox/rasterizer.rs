use gooey_core::{
    euclid::{Box2D, Length, Point2D, Rect, Size2D, Vector2D},
    styles::{Alignment, ForegroundColor},
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::MouseButton, EventStatus, Rasterizer, Renderer, WidgetRasterizer,
};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use crate::{
    button::ButtonColor,
    checkbox::{Checkbox, CheckboxCommand, CheckboxTransmogrifier, InternalCheckboxEvent},
};

const BUTTON_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for CheckboxTransmogrifier {
    type State = ();
    type Widget = Checkbox;

    fn receive_command(
        &self,
        _command: CheckboxCommand,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

#[derive(Clone, Default, Debug)]
pub struct LayoutState {
    content_size: Size2D<f32, Points>,
    checkbox_size: Size2D<f32, Points>,
    label_size: Size2D<f32, Points>,
    label: PreparedText,
}

fn calculate_layout<R: Renderer>(
    context: &TransmogrifierContext<'_, CheckboxTransmogrifier, Rasterizer<R>>,
    renderer: &R,
    size: Size2D<f32, Points>,
) -> LayoutState {
    // Determine the checkbox size by figuring out the line height
    let line_height = renderer.measure_text("m", context.style).height();
    let checkbox_size = Size2D::from_lengths(line_height, line_height);

    // Measure the label, allowing the text to wrap within the remaining space.
    let label_size = Size2D::new(
        (size.width - checkbox_size.width - BUTTON_PADDING.get()).max(0.),
        size.height,
    );
    let label = Text::span(&context.widget.label, context.style.clone())
        .wrap(renderer, TextWrap::MultiLine { size: label_size });

    let label_size = label.size();

    LayoutState {
        content_size: (label_size.to_vector()
            + Vector2D::new(checkbox_size.width + BUTTON_PADDING.get(), 0.))
        .to_size(),
        checkbox_size,
        label_size,
        label,
    }
}

impl<R: Renderer> WidgetRasterizer<R> for CheckboxTransmogrifier {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        if let Some(scene) = context.frontend.renderer() {
            let layout = calculate_layout(&context, scene, scene.size());
            let (checkbox_rect, label_rect) = match context
                .style
                .get::<Alignment>()
                .copied()
                .unwrap_or_default()
            {
                Alignment::Left | Alignment::Center => {
                    // Checkbox to the left
                    (
                        Rect::from_size(layout.checkbox_size),
                        Rect::new(
                            Point2D::new(layout.checkbox_size.width + BUTTON_PADDING.get(), 0.),
                            layout.label_size,
                        ),
                    )
                }
                Alignment::Right => {
                    // Checkbox to the right
                    (
                        Rect::new(
                            Point2D::new(layout.label_size.width + BUTTON_PADDING.get(), 0.),
                            layout.checkbox_size,
                        ),
                        Rect::from_size(layout.label_size),
                    )
                }
            };

            scene.fill_rect::<ButtonColor>(&checkbox_rect, context.style);

            if context.widget.checked {
                // Draw a simple X for now
                let check_box = checkbox_rect
                    .inflate(-BUTTON_PADDING.get(), -BUTTON_PADDING.get())
                    .to_box2d();
                // Round the x to pixel boundaries
                let check_box_pixels = check_box * scene.scale();
                let check_box =
                    Box2D::new(check_box_pixels.min.floor(), check_box_pixels.max.floor())
                        / scene.scale();
                scene.stroke_line(check_box.min, check_box.max, context.style);
                scene.stroke_line(
                    Point2D::new(check_box.max.x, check_box.min.y),
                    Point2D::new(check_box.min.x, check_box.max.y),
                    context.style,
                )
            }

            layout
                .label
                .render_within::<ForegroundColor, _>(scene, label_rect, context.style);
        }
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        // Always render a rect
        context
            .frontend
            .renderer()
            .map_or_else(Size2D::default, |renderer| {
                let renderer_size = renderer.size();
                let layout = calculate_layout(
                    &context,
                    renderer,
                    Size2D::new(
                        constraints.width.unwrap_or(renderer_size.width),
                        constraints.height.unwrap_or(renderer_size.height),
                    ),
                );

                layout.content_size
            })
    }

    fn mouse_down(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        _location: Point2D<f32, Points>,
        _rastered_size: Size2D<f32, Points>,
    ) -> EventStatus {
        if button == MouseButton::Left {
            context.frontend.activate(context.registration.id());
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        }
    }

    fn mouse_drag(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) {
        if Rect::from_size(rastered_size).contains(location) {
            context.frontend.activate(context.registration.id());
        } else {
            context.frontend.blur();
        }
    }

    fn mouse_up(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _button: MouseButton,
        location: Option<Point2D<f32, Points>>,
        rastered_size: Size2D<f32, Points>,
    ) {
        if location
            .map(|location| Rect::new(Point2D::default(), rastered_size).contains(location))
            .unwrap_or_default()
        {
            if let Some(widget) = context
                .frontend
                .ui
                .widget_state(context.registration.id().id)
            {
                widget
                    .channels::<Self::Widget>()
                    .unwrap()
                    .post_event(InternalCheckboxEvent::Clicked);
            }
        }
        context.frontend.deactivate();
    }
}
