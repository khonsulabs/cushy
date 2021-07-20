use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D, Vector2D},
    styles::TextColor,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::MouseButton, EventStatus, Rasterizer, Renderer, WidgetRasterizer,
};

use crate::button::{
    Button, ButtonColor, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent,
};

const BUTTON_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ButtonTransmogrifier {
    type State = ();
    type Widget = Button;

    fn receive_command(
        &self,
        _command: ButtonCommand,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for ButtonTransmogrifier {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        if let Some(scene) = context.frontend.renderer() {
            scene.fill_rect::<ButtonColor>(&scene.bounds(), context.style);

            let text_size = scene.measure_text(&context.widget.label, context.style);

            let center = scene.bounds().center();
            scene.render_text::<TextColor>(
                &context.widget.label,
                center - Vector2D::from_lengths(text_size.width, text_size.height()) / 2.
                    + Vector2D::from_lengths(Length::default(), text_size.ascent),
                context.style,
            );
        }
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .renderer()
            .map_or_else(Size2D::default, |scene| {
                // TODO should be wrapped width
                let text_size = scene.measure_text(&context.widget.label, context.style);
                (Vector2D::from_lengths(text_size.width, text_size.height())
                    + Vector2D::from_lengths(BUTTON_PADDING * 2., BUTTON_PADDING * 2.))
                .to_size()
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
                    .post_event(InternalButtonEvent::Clicked);
            }
        }
        context.frontend.deactivate();
    }
}
