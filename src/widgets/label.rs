//! A read-only text widget.

use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoUnsigned, Point, ScreenScale, Size};
use kludgine::text::{MeasuredText, Text, TextOrigin};

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

        let size = context.gfx.region().size;
        let center = Point::from(size) / 2;
        let styles = context.query_styles(&[&TextColor]);

        if let Some(measured) = &self.prepared_text {
            context
                .gfx
                .draw_measured_text(measured, TextOrigin::Center, center, None, None);
        } else {
            let text_color = styles.get(&TextColor, context);
            self.text.map(|contents| {
                context.gfx.draw_text(
                    Text::new(contents, text_color)
                        .wrap_at(size.width)
                        .origin(TextOrigin::Center),
                    center,
                    None,
                    None,
                );
            });
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let styles = context.query_styles(&[&TextColor, &IntrinsicPadding]);
        let padding = styles
            .get(&IntrinsicPadding, context)
            .into_px(context.gfx.scale())
            .into_unsigned();
        let color = styles.get(&TextColor, context);
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.text.map(|contents| {
            let measured = context
                .gfx
                .measure_text(Text::new(contents, color).wrap_at(width));
            let mut size = measured.size.try_cast().unwrap_or_default();
            size += padding * 2;
            self.prepared_text = Some(measured);
            size
        })
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
