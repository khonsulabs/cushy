//! A read-only text widget.

use std::fmt::Write;

use figures::units::{Px, UPx};
use figures::{Point, Round, Size};
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{CanRenderTo, Color, DrawableExt};

use crate::context::{GraphicsContext, LayoutContext, Trackable};
use crate::styles::components::TextColor;
use crate::value::{Dynamic, Generation, IntoReadOnly, ReadOnly, Value};
use crate::widget::{Widget, WidgetInstance};
use crate::window::WindowLocal;
use crate::ConstraintLimit;

/// A read-only text widget.
#[derive(Debug)]
pub struct Label<T> {
    /// The contents of the label.
    pub display: ReadOnly<T>,
    displayed: String,
    prepared_text: WindowLocal<(MeasuredText<Px>, Option<Generation>, Px, Color)>,
}

impl<T> Label<T>
where
    T: std::fmt::Debug + std::fmt::Display + Send + 'static,
{
    /// Returns a new label that displays `text`.
    pub fn new(text: impl IntoReadOnly<T>) -> Self {
        Self {
            display: text.into_read_only(),
            displayed: String::new(),
            prepared_text: WindowLocal::default(),
        }
    }

    fn prepared_text(
        &mut self,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
        color: Color,
        width: Px,
    ) -> &MeasuredText<Px> {
        let check_generation = self.display.generation();
        match self.prepared_text.get(context) {
            Some((prepared, prepared_generation, prepared_width, prepared_color))
                if prepared.can_render_to(&context.gfx)
                    && *prepared_generation == check_generation
                    && *prepared_color == color
                    && *prepared_width == width => {}
            _ => {
                context.apply_current_font_settings();
                let measured = self.display.map(|text| {
                    self.displayed.clear();
                    if let Err(err) = write!(&mut self.displayed, "{text}") {
                        tracing::error!("Error invoking Display: {err}");
                    }
                    context
                        .gfx
                        .measure_text(Text::new(&self.displayed, color).wrap_at(width))
                });
                self.prepared_text
                    .set(context, (measured, check_generation, width, color));
            }
        }

        self.prepared_text
            .get(context)
            .map(|(prepared, _, _, _)| prepared)
            .expect("always initialized")
    }
}

impl<T> Widget for Label<T>
where
    T: std::fmt::Debug + std::fmt::Display + Send + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.display.invalidate_when_changed(context);

        let size = context.gfx.region().size;
        let center = Point::from(size) / 2;
        let text_color = context.get(&TextColor);

        let prepared_text = self.prepared_text(context, text_color, size.width);

        context.gfx.draw_measured_text(
            prepared_text.translate_by(center.round()),
            TextOrigin::Center,
        );
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let color = context.get(&TextColor);
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        let prepared = self.prepared_text(context, color, width);

        prepared.size.try_cast().unwrap_or_default().ceil()
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_tuple("Label").field(&self.display).finish()
    }

    fn unmounted(&mut self, context: &mut crate::context::EventContext<'_, '_>) {
        self.prepared_text.clear_for(context);
    }
}

macro_rules! impl_make_widget {
    ($($type:ty => $kind:ty),*) => {
        $(impl crate::widget::MakeWidgetWithTag for $type {
            fn make_with_tag(self, id: crate::widget::WidgetTag) -> WidgetInstance {
                Label::<$kind>::new(self).make_with_tag(id)
            }
        })*
    };
}

impl_make_widget!(
    &'_ str => String,
    String => String,
    Dynamic<String> => String,
    Dynamic<&'static str> => &'static str,
    Value<String> => String,
    ReadOnly<String> => String
);
