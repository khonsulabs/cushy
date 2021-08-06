use gooey_core::{
    euclid::{Length, Point2D, Rect, Size2D},
    styles::{Style, TextColor},
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::MouseButton, ContentArea, EventStatus, Rasterizer, Renderer, WidgetRasterizer,
};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use crate::button::{Button, ButtonCommand, ButtonTransmogrifier, InternalButtonEvent};

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
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        if let Some(renderer) = context.frontend.renderer() {
            let wrapped = wrap_text(
                &context.widget.label,
                context.style,
                renderer,
                Length::new(content_area.size.content.width),
            );

            wrapped.render_within::<TextColor, _>(renderer, content_area.bounds(), context.style);
        }
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .renderer()
            .map_or_else(Size2D::default, |renderer| {
                wrap_text(
                    &context.widget.label,
                    context.style,
                    renderer,
                    Length::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                )
                .size()
            })
    }

    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
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
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
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
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
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

fn wrap_text<R: Renderer>(
    label: &str,
    style: &Style,
    renderer: &R,
    width: Length<f32, Points>,
) -> PreparedText {
    let text = Text::span(label, style.clone());
    text.wrap(renderer, TextWrap::SingleLine { width }, Some(style))
}
