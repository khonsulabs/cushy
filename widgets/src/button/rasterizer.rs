use gooey_core::{
    figures::{Figure, Point, Rectlike, Size},
    styles::{Style, TextColor},
    Scaled, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{
    winit::event::{ElementState, MouseButton, ScanCode, VirtualKeyCode},
    ContentArea, EventStatus, Rasterizer, Renderer, TransmogrifierContextExt, WidgetRasterizer,
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
    fn should_accept_focus(
        &self,
        _context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) -> bool {
        true
    }

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
                Figure::new(content_area.size.content.width),
            );

            wrapped.render_within::<TextColor, _>(
                renderer,
                content_area.content_bounds(),
                context.style,
            );
        }
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        context
            .frontend
            .renderer()
            .map_or_else(Size::default, |renderer| {
                wrap_text(
                    &context.widget.label,
                    context.style,
                    renderer,
                    Figure::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                )
                .size()
            })
    }

    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        _location: Point<f32, Scaled>,
        _area: &ContentArea,
    ) -> EventStatus {
        if button == MouseButton::Left {
            context.activate();
            EventStatus::Processed
        } else {
            EventStatus::Ignored
        }
    }

    fn mouse_drag(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) {
        if area.bounds().contains(location) {
            context.activate();
        } else {
            context.deactivate();
        }
    }

    fn mouse_up(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _button: MouseButton,
        location: Option<Point<f32, Scaled>>,
        area: &ContentArea,
    ) {
        context.deactivate();
        if location
            .map(|location| area.bounds().contains(location))
            .unwrap_or_default()
        {
            context.channels.post_event(InternalButtonEvent::Clicked);
            context.focus();
        }
    }

    fn keyboard(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        _scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventStatus {
        match keycode {
            Some(VirtualKeyCode::NumpadEnter | VirtualKeyCode::Return | VirtualKeyCode::Space) => {
                if matches!(state, ElementState::Pressed) {
                    context.activate();
                } else {
                    context.deactivate();
                    context.channels.post_event(InternalButtonEvent::Clicked);
                }
                EventStatus::Processed
            }
            _ => EventStatus::Ignored,
        }
    }
}

fn wrap_text<R: Renderer>(
    label: &str,
    style: &Style,
    renderer: &R,
    width: Figure<f32, Scaled>,
) -> PreparedText {
    let text = Text::span(label, style.clone());
    text.wrap(renderer, TextWrap::SingleLine { width }, Some(style))
}
