use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Size};
use kludgine::text::{Text, TextOrigin};

use crate::context::GraphicsContext;
use crate::styles::components::TextColor;
use crate::value::{IntoValue, Value};
use crate::widget::Widget;

/// A read-only text widget.
#[derive(Debug)]
pub struct Label {
    /// The contents of the label.
    pub text: Value<String>,
}

impl Label {
    /// Returns a new label that displays `text`.
    pub fn new(text: impl IntoValue<String>) -> Self {
        Self {
            text: text.into_value(),
        }
    }
}

impl Widget for Label {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.text.redraw_when_changed(context);

        let center = Point::from(context.graphics.size()) / 2;
        let styles = context.query_style(&[&TextColor]);
        let width = context.graphics.size().width;
        self.text.map(|contents| {
            context.graphics.draw_text(
                Text::new(contents, styles.get_or_default(&TextColor))
                    .origin(TextOrigin::Center)
                    .wrap_at(width),
                center,
                None,
                None,
            );
        });
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.text.map(|contents| {
            context
                .graphics
                .measure_text(Text::from(contents).wrap_at(width))
                .size
                .try_cast()
                .unwrap_or_default()
        })
    }
}
