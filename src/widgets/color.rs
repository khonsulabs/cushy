//! Widgets for selecting colors.
use std::ops::Range;

use figures::units::{Lp, Px};
use figures::{FloatConversion, Point, Rect, Round, ScreenScale, Zero};
use intentional::Cast;
use kludgine::app::winit::event::{DeviceId, MouseButton};
use kludgine::shapes::{self, FillOptions, PathBuilder, Shape, StrokeOptions};
use kludgine::{Color, DrawableExt, Origin};

use crate::animation::ZeroToOne;
use crate::context::{EventContext, GraphicsContext};
use crate::styles::components::{HighlightColor, OutlineColor, TextColor};
use crate::styles::{ColorExt, ColorSource};
use crate::value::{Destination, Dynamic, IntoValue, Source, Value};
use crate::widget::{EventHandling, Widget, HANDLED};

/// A widget that selects a [`ColorSource`].
#[derive(Debug)]
pub struct ColorSourcePicker {
    /// The currently selected hue and saturation.
    pub value: Dynamic<ColorSource>,
    /// The lightness value to render the color at.
    pub lightness: Value<ZeroToOne>,
    visible_rect: Rect<Px>,
    hue_is_360: bool,
}

impl ColorSourcePicker {
    /// Returns a new color picker that updates `value` when a new value is
    /// selected.
    #[must_use]
    pub fn new(value: Dynamic<ColorSource>) -> Self {
        Self {
            value,
            lightness: Value::Constant(ZeroToOne::new(0.5)),
            visible_rect: Rect::default(),
            hue_is_360: false,
        }
    }

    /// Sets the ligntness to render the color picker using.
    #[must_use]
    pub fn lightness(mut self, lightness: impl IntoValue<ZeroToOne>) -> Self {
        self.lightness = lightness.into_value();
        self
    }

    fn update_from_mouse(&mut self, location: Point<Px>) {
        let relative = (location - self.visible_rect.origin)
            .clamp(Point::ZERO, Point::from(self.visible_rect.size));

        let (is_360, hue) = if relative.x == self.visible_rect.size.width {
            (true, 360.)
        } else {
            (
                false,
                relative.x.into_float() / self.visible_rect.size.width.into_float() * 360.,
            )
        };
        self.hue_is_360 = is_360;

        let saturation =
            ZeroToOne::new(relative.y.into_float() / self.visible_rect.size.height.into_float())
                .one_minus();

        self.value.set(ColorSource::new(hue, saturation));
    }
}

impl Widget for ColorSourcePicker {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let loupe_size = Lp::mm(3).into_px(context.gfx.scale());
        let size = context.gfx.region().size;

        let value = self.value.get_tracking_redraw(context);
        let value_pos = self.visible_rect.origin
            + Point::new(
                if self.hue_is_360 {
                    self.visible_rect.size.width
                } else {
                    self.visible_rect.size.width * value.hue.into_positive_degrees() / 360.
                },
                self.visible_rect.size.height * *value.saturation.one_minus(),
            );

        let lightness = self.lightness.get_tracking_redraw(context);
        let value_color = value.color(lightness);

        let outline_color = if context.focused(true) {
            context.get(&HighlightColor)
        } else {
            context.get(&OutlineColor)
        };

        let options = StrokeOptions::lp_wide(Lp::points(1))
            .colored(outline_color)
            .into_px(context.gfx.scale());
        self.visible_rect = Rect::new(
            Point::squared(options.line_width / 2) + loupe_size / 2,
            size - Point::squared(options.line_width) - loupe_size,
        );

        let max_steps = (self.visible_rect.size.width / 2).floor().get();
        let steps = (self.visible_rect.size.width / 2)
            .floor()
            .get()
            .min(max_steps);

        let step_size = self.visible_rect.size.width / steps;
        let hue_step_size = 360. / steps.cast::<f32>();

        let mut x = self.visible_rect.origin.x;
        let mut hue = 0.;

        for step in 0..steps {
            let end = if step == steps - 1 {
                self.visible_rect.origin.x + self.visible_rect.size.width
            } else {
                x + step_size
            };
            let end_hue = hue + hue_step_size;
            draw_gradient_segment(
                Point::new(x, self.visible_rect.origin.y),
                end,
                self.visible_rect.size.height,
                hue..end_hue,
                lightness,
                context,
            );
            x = end;
            hue = end_hue;
        }

        context
            .gfx
            .draw_shape(&Shape::stroked_rect(self.visible_rect, options));

        // Draw the loupe
        context.gfx.draw_shape(
            Shape::filled_circle(loupe_size / 2, value_color, Origin::Center)
                .translate_by(value_pos),
        );
        let loupe_color = value_color.most_contrasting(&[outline_color, context.get(&TextColor)]);
        context.gfx.draw_shape(
            Shape::stroked_circle(loupe_size / 2, Origin::Center, options.colored(loupe_color))
                .translate_by(value_pos),
        );
    }

    fn hit_test(&mut self, location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        self.visible_rect.contains(location)
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        self.update_from_mouse(location);
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.update_from_mouse(location);
    }
}

fn draw_gradient_segment(
    start: Point<Px>,
    end: Px,
    height: Px,
    hue: Range<f32>,
    lightness: ZeroToOne,
    context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
) {
    let mid_left = (
        Point::new(start.x, start.y + height / 2),
        ColorSource::new(hue.start, ZeroToOne::new(0.5)).color(lightness),
    );
    let mid_right = (
        Point::new(end, start.y + height / 2),
        ColorSource::new(hue.end, ZeroToOne::new(0.5)).color(lightness),
    );

    context.gfx.draw_shape(
        &PathBuilder::new((
            start,
            ColorSource::new(hue.start, ZeroToOne::ONE).color(lightness),
        ))
        .line_to((
            Point::new(end, start.y),
            ColorSource::new(hue.end, ZeroToOne::ONE).color(lightness),
        ))
        .line_to(mid_right)
        .line_to(mid_left)
        .close()
        .fill_opt(
            Color::WHITE,
            &FillOptions::DEFAULT.with_sweep_orientation(shapes::Orientation::Horizontal),
        ),
    );

    context.gfx.draw_shape(
        &PathBuilder::new(mid_left)
            .line_to(mid_right)
            .line_to((
                Point::new(end, start.y + height),
                ColorSource::new(hue.end, ZeroToOne::ZERO).color(lightness),
            ))
            .line_to((
                Point::new(start.x, start.y + height),
                ColorSource::new(hue.start, ZeroToOne::ZERO).color(lightness),
            ))
            .close()
            .fill_opt(
                Color::WHITE,
                &FillOptions::DEFAULT.with_sweep_orientation(shapes::Orientation::Horizontal),
            ),
    );
}
