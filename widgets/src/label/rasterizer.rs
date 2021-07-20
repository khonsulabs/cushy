use gooey_core::{
    euclid::{Length, Point2D, Size2D, Vector2D},
    styles::Alignment,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, Renderer, WidgetRasterizer};
use gooey_text::{wrap::TextWrap, Text};

use crate::label::{Label, LabelCommand, LabelTransmogrifier};

const LABEL_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for LabelTransmogrifier {
    type State = ();
    type Widget = Label;

    fn receive_command(
        &self,
        _command: LabelCommand,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for LabelTransmogrifier {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        if let Some(scene) = context.frontend.renderer() {
            // TODO switch to borrows?
            let text = Text::span(&context.widget.label, context.style.clone());
            let wrapped = text.wrap(scene, TextWrap::SingleLine {
                max_width: Length::new(scene.size().width) - LABEL_PADDING * 2.,
                alignment: Alignment::Center,
                truncate: true,
            });
            let text_size = wrapped.size();
            wrapped.render(
                scene,
                Point2D::new(0., (scene.size().height - text_size.height) / 2.),
                true,
            );
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
            .map_or_else(Size2D::default, |scene| {
                let text = Text::span(&context.widget.label, context.style.clone());
                let wrapped = text.wrap(scene, TextWrap::SingleLine {
                    max_width: Length::new(constraints.width.unwrap_or_else(|| scene.size().width))
                        - LABEL_PADDING * 2.,
                    alignment: Alignment::Center,
                    truncate: true,
                });
                (wrapped.size().to_vector()
                    + Vector2D::from_lengths(LABEL_PADDING * 2., LABEL_PADDING * 2.))
                .to_size()
            })
    }
}
