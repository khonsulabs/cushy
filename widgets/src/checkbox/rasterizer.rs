use gooey_core::{
    euclid::{Point2D, Rect, Size2D, Vector2D},
    styles::ForegroundColor,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::MouseButton, EventStatus, Rasterizer, Renderer, WidgetRasterizer,
};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use crate::{
    button::ButtonColor,
    checkbox::{
        Checkbox, CheckboxCommand, CheckboxTransmogrifier, InternalCheckboxEvent, LABEL_PADDING,
    },
};

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
        (size.width - checkbox_size.width - LABEL_PADDING.get()).max(0.),
        size.height,
    );
    let label = Text::span(&context.widget.label, context.style.clone())
        .wrap(renderer, TextWrap::MultiLine { size: label_size });

    let label_size = label.size();

    LayoutState {
        content_size: (label_size.to_vector()
            + Vector2D::new(checkbox_size.width + LABEL_PADDING.get(), 0.))
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
            let checkbox_rect = Rect::from_size(layout.checkbox_size);
            let label_rect = Rect::new(
                Point2D::new(layout.checkbox_size.width + LABEL_PADDING.get(), 0.),
                layout.label_size,
            );

            scene.fill_rect::<ButtonColor>(&checkbox_rect, context.style);

            if context.widget.checked {
                // Fill a square in the middle with the mark.
                let check_box = checkbox_rect.inflate(
                    -checkbox_rect.size.width / 3.,
                    -checkbox_rect.size.width / 3.,
                );
                scene.fill_rect::<ForegroundColor>(&check_box, context.style);
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
