use gooey_core::{
    euclid::{Length, Point2D, Size2D, Vector2D},
    renderer::Renderer,
    styles::{ForegroundColor, Points, Srgba, Style},
    Transmogrifier,
};
use gooey_rasterizer::{
    winit::event::MouseButton, EventStatus, RasterContext, Rasterizer, WidgetRasterizer,
};

use super::InternalButtonEvent;
use crate::button::{Button, ButtonCommand, ButtonTransmogrifier};

const BUTTON_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for ButtonTransmogrifier {
    type State = ();
    type Widget = Button;

    fn receive_command(
        &self,
        _state: &mut Self::State,
        _command: ButtonCommand,
        _widget: &Self::Widget,
        frontend: &Rasterizer<R>,
    ) {
        frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for ButtonTransmogrifier {
    fn render(&self, context: RasterContext<Self, R>) {
        if let Some(scene) = context.rasterizer.renderer() {
            scene.fill_rect(
                &scene.bounds(),
                &Style::new().with(ForegroundColor(Srgba::new(0., 1., 0., 1.).into())),
            );

            let text_size = scene.measure_text(&context.widget.label, &Style::default());

            let center = scene.bounds().center();
            scene.render_text(
                &context.widget.label,
                center - Vector2D::from_lengths(text_size.width, text_size.height()) / 2.
                    + Vector2D::from_lengths(Length::default(), text_size.ascent),
                &Style::default(),
            );
        }
    }

    fn content_size(
        &self,
        context: RasterContext<Self, R>,
        _constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        if let Some(scene) = context.rasterizer.renderer() {
            // TODO should be wrapped width
            let text_size = scene.measure_text(&context.widget.label, &Style::default());
            (Vector2D::from_lengths(text_size.width, text_size.height())
                + Vector2D::from_lengths(BUTTON_PADDING * 2., BUTTON_PADDING * 2.))
            .to_size()
        } else {
            Size2D::default()
        }
    }

    fn mouse_down(
        &self,
        _context: RasterContext<Self, R>,
        _location: Point2D<f32, Points>,
        button: MouseButton,
    ) -> EventStatus {
        if button == MouseButton::Left {
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        }
    }

    fn mouse_up(
        &self,
        context: RasterContext<Self, R>,
        _location: Option<Point2D<f32, Points>>,
        _button: MouseButton,
    ) {
        // TODO check for location to be contained
        if let Some(widget) = context
            .rasterizer
            .ui
            .widget_state(context.registration.id().id)
        {
            widget
                .channels::<Self::Widget>()
                .unwrap()
                .post_event(InternalButtonEvent::Clicked);
        }
    }
}
