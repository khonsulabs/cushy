use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, ScreenScale, Size};
use kludgine::text::{MeasuredText, Text, TextOrigin};

use crate::context::GraphicsContext;
use crate::styles::components::{IntrinsicPadding, TextColor};
use crate::value::{IntoValue, Value};
use crate::widget::Widget;

/// A read-only text widget.
#[derive(Debug)]
pub struct Label {
    /// The contents of the label.
    pub text: Value<String>,
    prepared_text: Option<MeasuredText<Px>>,
}

impl Label {
    /// Returns a new label that displays `text`.
    pub fn new(text: impl IntoValue<String>) -> Self {
        Self {
            text: text.into_value(),
            prepared_text: None,
        }
    }
}

impl Widget for Label {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.text.redraw_when_changed(context);

        let size = context.graphics.region().size;
        let center = Point::from(size) / 2;
        let styles = context.query_styles(&[&TextColor]);

        if let Some(measured) = &self.prepared_text {
            context
                .graphics
                .draw_measured_text(measured, TextOrigin::Center, center, None, None);
        } else {
            self.text.map(|contents| {
                context.graphics.draw_text(
                    Text::new(contents, styles.get_or_default(&TextColor))
                        .wrap_at(size.width)
                        .origin(TextOrigin::Center),
                    center,
                    None,
                    None,
                );
            });
        }
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .query_style(&IntrinsicPadding)
            .into_px(context.graphics.scale())
            .into_unsigned();
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.text.map(|contents| {
            let measured = context
                .graphics
                .measure_text(Text::from(contents).wrap_at(width));
            let mut size = measured.size.try_cast().unwrap_or_default();
            size += padding * 2;
            self.prepared_text = Some(measured);
            size
        })
    }
}
