//! Widgets for selecting colors.
use std::ops::Range;

use figures::units::{Lp, Px};
use figures::{FloatConversion, Point, Rect, Round, ScreenScale, Size, Zero};
use intentional::Cast;
use kludgine::app::winit::event::MouseButton;
use kludgine::shapes::{self, CornerRadii, FillOptions, PathBuilder, Shape, StrokeOptions};
use kludgine::{Color, DrawableExt, Origin};

use crate::animation::ZeroToOne;
use crate::context::{EventContext, GraphicsContext};
use crate::styles::components::{HighlightColor, OutlineColor, TextColor};
use crate::styles::{ColorExt, ColorSource, Hsl};
use crate::value::{Destination, Dynamic, ForEachCloned, IntoDynamic, IntoValue, Source, Value};
use crate::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetTag, HANDLED,
};
use crate::window::DeviceId;

/// A color picker that selects an [`Hsl`] color.
#[derive(Debug)]
pub struct HslPicker {
    source: Dynamic<ColorSource>,
    lightness: Dynamic<ZeroToOne>,
}

impl HslPicker {
    /// Returns a new color picker that updates `hsl` when a new value is
    /// chosen.
    #[must_use]
    pub fn new(hsl: Dynamic<Hsl>) -> Self {
        let source = hsl.map_each(|hsl| hsl.source);
        let lightness = hsl.map_each(|hsl| hsl.lightness);

        (&source, &lightness)
            .for_each_cloned(move |(source, lightness)| hsl.set(Hsl { source, lightness }))
            .persist();

        Self { source, lightness }
    }
}

impl MakeWidgetWithTag for HslPicker {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        ColorSourcePicker::new(self.source)
            .lightness(self.lightness.clone())
            .make_with_tag(tag)
            .expand()
            .and(LightnessPicker::new(self.lightness).height(Lp::points(24)))
            .into_rows()
            .gutter(Px::ZERO)
            .make_widget()
    }
}

/// A widget that selects between completely dark and completely light by
/// utilizing a back-to-white gradient.
#[derive(Debug)]
pub struct LightnessPicker {
    value: Dynamic<ZeroToOne>,
    visible_rect: Rect<Px>,
}

impl LightnessPicker {
    /// Returns a new picker that updates `value` when a new lightness is
    /// selected.
    pub fn new(value: impl IntoDynamic<ZeroToOne>) -> Self {
        Self {
            value: value.into_dynamic(),
            visible_rect: Rect::default(),
        }
    }

    fn update_from_mouse(&mut self, location: Point<Px>) {
        let relative = (location - self.visible_rect.origin)
            .clamp(Point::ZERO, Point::from(self.visible_rect.size));

        let lightness = relative.x.into_float() / self.visible_rect.size.width.into_float();

        self.value.set(ZeroToOne::new(lightness));
    }
}

impl Widget for LightnessPicker {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        const STEPS: u8 = 10;
        let loupe_size = Lp::mm(3).into_px(context.gfx.scale());
        let size = context.gfx.region().size;

        let outline_color = if context.focused(true) {
            context.get(&HighlightColor)
        } else {
            context.get(&OutlineColor)
        };

        let options = StrokeOptions::lp_wide(Lp::points(1))
            .colored(outline_color)
            .into_px(context.gfx.scale());
        self.visible_rect = Rect::new(
            Point::squared(options.line_width) + Point::new(loupe_size / 2, Px::ZERO),
            size - Size::squared(options.line_width * 2) - Size::new(loupe_size, Px::ZERO),
        );

        let (top_left, bottom_right) = self.visible_rect.extents();

        // Unfortunately, drawing a single back-to-white gradient doesn't work
        // visually due to srgb mapping.
        let mut x = top_left.x;
        let top = top_left.y;
        let bottom = bottom_right.y;
        let mut lightness = ZeroToOne::ZERO;
        let step_width = self.visible_rect.size.width / i32::from(STEPS);
        let step_lightness = 1.0 / f32::from(STEPS);
        let mut gray = Color::BLACK;
        for step in 0..STEPS {
            let (end_x, end_gray) = if step == STEPS - 1 {
                (bottom_right.x, Color::WHITE)
            } else {
                lightness = ZeroToOne::new(*lightness + step_lightness);
                (x + step_width, ColorSource::new(0., 0.).color(lightness))
            };
            context.gfx.draw_shape(
                &PathBuilder::new((Point::new(x, top), gray))
                    .line_to((Point::new(end_x, top), end_gray))
                    .line_to((Point::new(end_x, bottom), end_gray))
                    .line_to((Point::new(x, bottom), gray))
                    .close()
                    .filled(),
            );
            x = end_x;
            gray = end_gray;
        }

        context.gfx.draw_shape(&Shape::stroked_rect(
            self.visible_rect.inset(-options.line_width / 2),
            options,
        ));

        let value = self.value.get_tracking_redraw(context);
        let value_x = self.visible_rect.origin.x + self.visible_rect.size.width * *value;
        let loupe_rect = Rect::new(
            Point::new(value_x - loupe_size / 2, options.line_width / 2),
            Size::new(loupe_size, size.height - options.line_width),
        );
        let selected_gray = ColorSource::new(0., 0.).color(value);
        context.gfx.draw_shape(&Shape::filled_round_rect(
            loupe_rect,
            CornerRadii::from(loupe_size),
            selected_gray,
        ));
        let loupe_color =
            selected_gray.most_contrasting(&[context.get(&OutlineColor), context.get(&TextColor)]);
        context.gfx.draw_shape(&Shape::stroked_round_rect(
            loupe_rect,
            CornerRadii::from(loupe_size),
            options.colored(loupe_color),
        ));
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_>,
    ) -> EventHandling {
        self.update_from_mouse(location);
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_>,
    ) {
        self.update_from_mouse(location);
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        true
    }
}

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
    pub fn new(value: impl IntoDynamic<ColorSource>) -> Self {
        Self {
            value: value.into_dynamic(),
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
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let loupe_size = Lp::mm(3).into_px(context.gfx.scale());
        let size = context.gfx.region().size;

        let outline_color = if context.focused(true) {
            context.get(&HighlightColor)
        } else {
            context.get(&OutlineColor)
        };

        let options = StrokeOptions::lp_wide(Lp::points(1))
            .colored(outline_color)
            .into_px(context.gfx.scale());
        self.visible_rect = Rect::new(
            Point::squared(options.line_width) + loupe_size / 2,
            size - Point::squared(options.line_width * 2) - loupe_size,
        );

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

        context.gfx.draw_shape(&Shape::stroked_rect(
            self.visible_rect.inset(-options.line_width / 2),
            options,
        ));

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

    fn hit_test(&mut self, location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        self.visible_rect.contains(location)
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_>,
    ) -> EventHandling {
        self.update_from_mouse(location);
        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: MouseButton,
        _context: &mut EventContext<'_>,
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
    context: &mut GraphicsContext<'_, '_, '_, '_>,
) {
    let vertical_slices = (height.get() / 16).clamp(3, 10);
    let slice_neight = height / vertical_slices;
    let slice_saturation = 1.0 / vertical_slices.cast::<f32>();

    let mut y = start.y;
    let mut saturation = ZeroToOne::ONE;
    for slice in 0..vertical_slices {
        let (bottom, bottom_saturation) = if slice + 1 == vertical_slices {
            (start.y + height, ZeroToOne::ZERO)
        } else {
            (
                y + slice_neight,
                ZeroToOne::new(*saturation - slice_saturation),
            )
        };

        context.gfx.draw_shape(
            &PathBuilder::new((
                Point::new(start.x, y),
                ColorSource::new(hue.start, saturation).color(lightness),
            ))
            .line_to((
                Point::new(end, y),
                ColorSource::new(hue.end, saturation).color(lightness),
            ))
            .line_to((
                Point::new(end, bottom),
                ColorSource::new(hue.end, bottom_saturation).color(lightness),
            ))
            .line_to((
                Point::new(start.x, bottom),
                ColorSource::new(hue.start, bottom_saturation).color(lightness),
            ))
            .close()
            .fill_opt(
                Color::WHITE,
                &FillOptions::DEFAULT.with_sweep_orientation(shapes::Orientation::Horizontal),
            ),
        );

        y += slice_neight;
        saturation = bottom_saturation;
    }
}
