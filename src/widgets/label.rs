//! A read-only text widget.

use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, ScreenScale, Size};
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::Color;

use crate::context::{GraphicsContext, LayoutContext};
use crate::styles::components::{IntrinsicPadding, TextColor};
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{MakeWidget, Widget, WidgetInstance};
use crate::ConstraintLimit;

/// A read-only text widget.
#[derive(Debug)]
pub struct Label {
    /// The contents of the label.
    pub text: Value<String>,
    prepared_text: Option<(MeasuredText<Px>, Px, Color)>,
}

impl Label {
    /// Returns a new label that displays `text`.
    pub fn new(text: impl IntoValue<String>) -> Self {
        Self {
            text: text.into_value(),
            prepared_text: None,
        }
    }

    fn prepared_text(
        &mut self,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
        color: Color,
        width: Px,
    ) -> &MeasuredText<Px> {
        match &self.prepared_text {
            Some((_, prepared_width, prepared_color))
                if *prepared_color == color && *prepared_width == width => {}
            _ => {
                let measured = self.text.map(|text| {
                    context
                        .gfx
                        .measure_text(Text::new(text, color).wrap_at(width))
                });
                self.prepared_text = Some((measured, width, color));
            }
        }

        self.prepared_text
            .as_ref()
            .map(|(prepared, _, _)| prepared)
            .expect("always initialized")
    }
}

impl Widget for Label {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.text.redraw_when_changed(context);

        let size = context.gfx.region().size;
        let center = Point::from(size) / 2;
        let text_color = context.get(&TextColor);

        let prepared_text = self.prepared_text(context, text_color, size.width);

        context
            .gfx
            .draw_measured_text(prepared_text, TextOrigin::Center, center, None, None);
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .get(&IntrinsicPadding)
            .into_px(context.gfx.scale())
            .into_unsigned();
        let color = context.get(&TextColor);
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        let prepared = self.prepared_text(context, color, width);

        let mut size = prepared.size.try_cast().unwrap_or_default();
        size += padding * 2;
        size
    }
}

macro_rules! impl_make_widget {
    ($($type:ty),*) => {
        $(impl MakeWidget for $type {
            fn make_widget(self) -> WidgetInstance {
                Label::new(self).make_widget()
            }
        })*
    };
}

impl_make_widget!(&'_ str, String, Value<String>, Dynamic<String>);
