use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Size};
use kludgine::text::TextOrigin;
use kludgine::Color;

use crate::context::Context;
use crate::graphics::Graphics;
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
    fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context<'_, '_>) {
        let center = Point::from(graphics.size()) / 2;
        if let Value::Dynamic(contents) = &mut self.contents {
            context.redraw_when_changed(contents);
        }
        let styles = context.query_style(&[&TextColor]);
        let width = graphics.size().width;
        self.contents.map(|contents| {
            graphics.draw_text(
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
        graphics: &mut Graphics<'_, '_, '_>,
        _context: &mut Context<'_, '_>,
    ) -> Size<UPx> {
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        self.contents.map(|contents| {
            graphics
                .measure_text(contents, Color::RED, Some(width))
                .size
                .try_cast()
                .unwrap_or_default()
        })
    }
}
