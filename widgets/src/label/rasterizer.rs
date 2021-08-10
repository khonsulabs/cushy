use gooey_core::{
    euclid::{Length, Size2D},
    styles::Style,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{ContentArea, Rasterizer, Renderer, WidgetRasterizer};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use super::LabelColor;
use crate::label::{Command, Label, LabelTransmogrifier};

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for LabelTransmogrifier {
    type State = ();
    type Widget = Label;

    fn receive_command(
        &self,
        _command: Command,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for LabelTransmogrifier {
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        if let Some(renderer) = context.frontend.renderer() {
            // TODO switch to borrows?
            let wrapped = wrap_text(
                &context.widget.label,
                context.style,
                renderer,
                Length::new(content_area.size.content.width),
            );
            wrapped.render_within::<LabelColor, _>(
                renderer,
                content_area.content_bounds(),
                context.style,
            );
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
                let wrapped = wrap_text(
                    &context.widget.label,
                    context.style,
                    renderer,
                    Length::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                );
                wrapped.size()
            })
    }
}

fn wrap_text<R: Renderer>(
    label: &Text,
    style: &Style,
    renderer: &R,
    width: Length<f32, Points>,
) -> PreparedText {
    label.wrap(
        renderer,
        TextWrap::MultiLine {
            size: Size2D::from_lengths(width, Length::new(renderer.size().height)),
        },
        Some(style),
    )
}
