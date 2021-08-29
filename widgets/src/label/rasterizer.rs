use gooey_core::{
    figures::{Figure, Size},
    styles::Style,
    Scaled, Transmogrifier, TransmogrifierContext,
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
                context.style(),
                renderer,
                Figure::new(content_area.size.content.width),
            );
            wrapped.render_within::<LabelColor, _>(
                renderer,
                content_area.content_bounds(),
                context.style(),
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
                let wrapped = wrap_text(
                    &context.widget.label,
                    context.style(),
                    renderer,
                    Figure::new(constraints.width.unwrap_or_else(|| renderer.size().width)),
                );
                wrapped.size()
            })
    }
}

fn wrap_text<R: Renderer>(
    label: &Text,
    style: &Style,
    renderer: &R,
    width: Figure<f32, Scaled>,
) -> PreparedText {
    label.wrap(
        renderer,
        TextWrap::MultiLine {
            size: Size::from_figures(width, Figure::new(renderer.size().height)),
        },
        Some(style),
    )
}
