use gooey_core::{
    euclid::{Length, Size2D, Vector2D},
    styles::Style,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, Renderer, WidgetRasterizer};
use gooey_text::{prepared::PreparedText, wrap::TextWrap, Text};

use super::LabelColor;
use crate::label::{Command, Label, LabelTransmogrifier};

const LABEL_PADDING: Length<f32, Points> = Length::new(5.);

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
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        if let Some(renderer) = context.frontend.renderer() {
            // TODO switch to borrows?
            let wrapped = wrap_text(
                &context.widget.label,
                context.style,
                renderer,
                Length::new(renderer.size().width),
            );
            wrapped.render_within::<LabelColor, _>(renderer, renderer.bounds(), context.style);
        }
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
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
                (wrapped.size().to_vector()
                    + Vector2D::from_lengths(LABEL_PADDING * 2., LABEL_PADDING * 2.))
                .to_size()
            })
    }
}

fn wrap_text<R: Renderer>(
    label: &str,
    style: &Style,
    renderer: &R,
    width: Length<f32, Points>,
) -> PreparedText {
    let text = Text::span(label, style.clone());
    text.wrap(renderer, TextWrap::MultiLine {
        size: Size2D::from_lengths(
            width - LABEL_PADDING * 2.,
            Length::new(renderer.size().height),
        ),
    })
}
