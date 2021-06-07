use gooey_core::{
    euclid::{Length, Size2D, Vector2D},
    renderer::Renderer,
    Points, Transmogrifier, TransmogrifierContext,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use crate::label::{Label, LabelColor, LabelCommand, LabelTransmogrifier};

const LABEL_PADDING: Length<f32, Points> = Length::new(5.);

impl<R: Renderer> Transmogrifier<Rasterizer<R>> for LabelTransmogrifier {
    type State = ();
    type Widget = Label;

    fn receive_command(
        &self,
        _command: LabelCommand,
        context: &mut TransmogrifierContext<Self, Rasterizer<R>>,
    ) {
        context.frontend.set_needs_redraw();
    }
}

impl<R: Renderer> WidgetRasterizer<R> for LabelTransmogrifier {
    fn render(&self, context: TransmogrifierContext<Self, Rasterizer<R>>) {
        if let Some(scene) = context.frontend.renderer() {
            let text_size = scene.measure_text(&context.widget.label, context.style);

            let center = scene.bounds().center();
            scene.render_text::<LabelColor>(
                &context.widget.label,
                center - Vector2D::from_lengths(text_size.width, text_size.height()) / 2.
                    + Vector2D::from_lengths(Length::default(), text_size.ascent),
                context.style,
            );
        }
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<Self, Rasterizer<R>>,
        _constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        if let Some(scene) = context.frontend.renderer() {
            // TODO should be wrapped width
            let text_size = scene.measure_text(&context.widget.label, context.style);
            (Vector2D::from_lengths(text_size.width, text_size.height())
                + Vector2D::from_lengths(LABEL_PADDING * 2., LABEL_PADDING * 2.))
            .to_size()
        } else {
            Size2D::default()
        }
    }
}
