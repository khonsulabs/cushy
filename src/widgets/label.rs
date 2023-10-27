use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Size};
use kludgine::text::TextOrigin;
use kludgine::Color;

use crate::context::GraphicsContext;
use crate::styles::TextColor;
use crate::widget::{IntoValue, Value, Widget};

#[derive(Debug)]
pub struct Label {
    pub contents: Value<String>,
}

impl Label {
    pub fn new(contents: impl IntoValue<String>) -> Self {
        Self {
            contents: contents.into_value(),
        }
    }
}

impl Widget for Label {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.contents.redraw_when_changed(context);

        let center = Point::from(context.graphics.size()) / 2;
        let styles = context.query_style(&[&TextColor]);
        let width = context.graphics.size().width;
        self.contents.map(|contents| {
            context.graphics.draw_text(
                contents,
                styles.get_or_default(&TextColor),
                TextOrigin::Center,
                center,
                None,
                None,
                Some(width),
            );
        });
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.contents.map(|contents| {
            context
                .graphics
                .measure_text(contents, Color::RED, Some(width))
                .size
                .try_cast()
                .unwrap_or_default()
        })
    }
}
