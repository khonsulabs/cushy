//! Widgets for selecting colors.
use std::ops::Range;

use figures::units::{Lp, Px, UPx};
use figures::{FloatConversion, Point, Rect, Round, ScreenScale, Size, Zero};
use intentional::Cast;
use kludgine::app::winit::event::MouseButton;
use kludgine::shapes::{self, CornerRadii, FillOptions, PathBuilder, Shape, StrokeOptions};
use kludgine::{Color, DrawableExt, Origin};

use crate::animation::{LinearInterpolate, PercentBetween, ZeroToOne};
use crate::context::{EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::{
    HighlightColor, OutlineColor, OutlineWidth, SurfaceColor, TextColor,
};
use crate::styles::{ColorExt, ColorSource, Hsl, Hsla};
use crate::value::{
    Destination, Dynamic, ForEachCloned, IntoDynamic, IntoReadOnly, IntoValue, MapEach, ReadOnly,
    Source, Value,
};
use crate::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetTag, HANDLED,
};
use crate::window::DeviceId;
use crate::ConstraintLimit;

/// A [`Color`] picker that allows selecting a color using individual red,
/// green, blue, and alpha [`ComponentPicker`]s.
pub struct RgbaPicker {
    color: Dynamic<Color>,
}

impl RgbaPicker {
    /// Returns a new picker that updates `color` when a new color is selected.
    pub fn new(color: impl IntoDynamic<Color>) -> Self {
        Self {
            color: color.into_dynamic(),
        }
    }
}

impl MakeWidgetWithTag for RgbaPicker {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        let red = self.color.map_each_cloned(Color::red);
        let green = self.color.map_each_cloned(Color::green);
        let blue = self.color.map_each_cloned(Color::blue);
        let alpha = self.color.map_each_cloned(Color::alpha);

        let color = self.color.clone();
        (&red, &green, &blue, &alpha)
            .for_each_cloned(move |(red, green, blue, alpha)| {
                color.set(Color::new(red, green, blue, alpha));
            })
            .persist();

        let red_picker = ComponentPicker::red(red);
        let green_picker = ComponentPicker::green(green);
        let blue_picker = ComponentPicker::blue(blue);
        let alpha_picker = ComponentPicker::alpha(alpha, self.color);

        red_picker
            .and(green_picker)
            .and(blue_picker)
            .and(alpha_picker)
            .into_rows()
            .make_with_tag(tag)
    }
}

/// A [`Color`] picker that allows selecting a color using individual red,
/// green, and blue [`ComponentPicker`]s.
pub struct RgbPicker {
    color: Dynamic<Color>,
}

impl RgbPicker {
    /// Returns a new picker that updates `color` when a new color is selected.
    pub fn new(color: impl IntoDynamic<Color>) -> Self {
        Self {
            color: color.into_dynamic(),
        }
    }
}

impl MakeWidgetWithTag for RgbPicker {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        let red = self.color.map_each_cloned(Color::red);
        let green = self.color.map_each_cloned(Color::green);
        let blue = self.color.map_each_cloned(Color::blue);

        (&red, &green, &blue)
            .for_each_cloned(move |(red, green, blue)| {
                self.color.set(Color::new(red, green, blue, 255));
            })
            .persist();

        let red_picker = ComponentPicker::red(red);
        let green_picker = ComponentPicker::green(green);
        let blue_picker = ComponentPicker::blue(blue);

        red_picker
            .and(green_picker)
            .and(blue_picker)
            .into_rows()
            .make_with_tag(tag)
    }
}

/// A picker for an [`Hsla`] color.
#[derive(Debug)]
pub struct HslaPicker {
    source: Dynamic<ColorSource>,
    lightness: Dynamic<ZeroToOne>,
    alpha: Dynamic<ZeroToOne>,
}

impl HslaPicker {
    /// Returns a new color picker that updates `hsla` when a new value is
    /// chosen.
    #[must_use]
    pub fn new(hsla: Dynamic<Hsla>) -> Self {
        let source = hsla.map_each(|hsla| hsla.hsl.source);
        let lightness = hsla.map_each(|hsla| hsla.hsl.lightness);
        let alpha = hsla.map_each(|hsla| hsla.alpha);

        (&source, &lightness, &alpha)
            .for_each_cloned(move |(source, lightness, alpha)| {
                hsla.set(Hsla {
                    hsl: Hsl { source, lightness },
                    alpha,
                });
            })
            .persist();

        Self {
            source,
            lightness,
            alpha,
        }
    }
}

impl MakeWidgetWithTag for HslaPicker {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        let preview_color = (&self.source, &self.lightness)
            .map_each(|(source, lightness)| source.color(*lightness));
        ColorSourcePicker::new(self.source)
            .lightness(self.lightness.clone())
            .make_with_tag(tag)
            .expand()
            .and(ComponentPicker::lightness(self.lightness))
            .and(ComponentPicker::alpha_f32(self.alpha, preview_color))
            .into_rows()
            .make_widget()
    }
}

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
            .and(ComponentPicker::lightness(self.lightness))
            .into_rows()
            .gutter(Px::ZERO)
            .make_widget()
    }
}

/// A component that can be picked in a [`ComponentPicker`].
pub trait ColorComponent: std::fmt::Debug + Send + 'static {
    /// Returns the color to display at the start of the component picker.
    fn start_color(&self) -> Color;
    /// Returns the color to display at the end of the component picker.
    fn end_color(&self) -> Color;
    /// Interpolate the color at the given percentage between the start and end
    /// colors.
    fn interpolate_color(&self, percent: ZeroToOne) -> Color;

    /// Returns the color to to display within the loupe.
    fn loupe_color(&self, percent: ZeroToOne) -> Option<Color> {
        Some(self.interpolate_color(percent))
    }

    /// Draws the background behind the color component.
    #[allow(unused_variables)]
    fn draw_background(&self, rect: Rect<Px>, graphics: &mut GraphicsContext<'_, '_, '_, '_>) {}
}

/// A [`ColorComponent`] that configures a [`ComponentPicker`] to pick the
/// "lightness" of a color.
///
/// The lightness component comes from computing the light's hue, saturation,
/// and lightness (HSL) components.
#[derive(Debug)]
pub struct Lightness;

impl ColorComponent for Lightness {
    fn start_color(&self) -> Color {
        Color::BLACK
    }

    fn end_color(&self) -> Color {
        Color::WHITE
    }

    fn interpolate_color(&self, percent: ZeroToOne) -> Color {
        ColorSource::new(0., 0.).color(percent)
    }
}

/// A [`ColorComponent`] that configures a [`ComponentPicker`] to pick the
/// red copmponent of a color.
#[derive(Debug)]
pub struct Red;

impl Red {
    fn build_color(red: ZeroToOne) -> Color {
        Color::new_f32(*red, 0., 0., 1.)
    }
}

impl ColorComponent for Red {
    fn start_color(&self) -> Color {
        Self::build_color(ZeroToOne::ZERO)
    }

    fn end_color(&self) -> Color {
        Self::build_color(ZeroToOne::ONE)
    }

    fn interpolate_color(&self, percent: ZeroToOne) -> Color {
        Self::build_color(percent)
    }
}

/// A [`ColorComponent`] that configures a [`ComponentPicker`] to pick the
/// green copmponent of a color.
#[derive(Debug)]
pub struct Green;

impl Green {
    fn build_color(green: ZeroToOne) -> Color {
        Color::new_f32(0., *green, 0., 1.)
    }
}

impl ColorComponent for Green {
    fn start_color(&self) -> Color {
        Self::build_color(ZeroToOne::ZERO)
    }

    fn end_color(&self) -> Color {
        Self::build_color(ZeroToOne::ONE)
    }

    fn interpolate_color(&self, percent: ZeroToOne) -> Color {
        Self::build_color(percent)
    }
}

/// A [`ColorComponent`] that configures a [`ComponentPicker`] to pick the
/// blue copmponent of a color.
#[derive(Debug)]
pub struct Blue;

impl Blue {
    fn build_color(blue: ZeroToOne) -> Color {
        Color::new_f32(0., 0., *blue, 1.)
    }
}

impl ColorComponent for Blue {
    fn start_color(&self) -> Color {
        Self::build_color(ZeroToOne::ZERO)
    }

    fn end_color(&self) -> Color {
        Self::build_color(ZeroToOne::ONE)
    }

    fn interpolate_color(&self, percent: ZeroToOne) -> Color {
        Self::build_color(percent)
    }
}

/// A [`ColorComponent`] that configures a [`ComponentPicker`] to pick the alpha
/// copmponent of a color.
#[derive(Debug)]
pub struct Alpha {
    color: ReadOnly<Color>,
}

impl Alpha {
    fn build_color(&self, alpha: ZeroToOne) -> Color {
        self.color.get().with_alpha_f32(*alpha)
    }
}

impl ColorComponent for Alpha {
    fn start_color(&self) -> Color {
        self.build_color(ZeroToOne::ZERO)
    }

    fn end_color(&self) -> Color {
        self.build_color(ZeroToOne::ONE)
    }

    fn interpolate_color(&self, percent: ZeroToOne) -> Color {
        self.build_color(percent)
    }

    fn loupe_color(&self, _percent: ZeroToOne) -> Option<Color> {
        None
    }

    fn draw_background(&self, rect: Rect<Px>, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let checker_size = Lp::points(8).into_px(context.gfx.scale()).ceil();
        let shape = Shape::filled_rect(
            Size::squared(checker_size).into(),
            context.theme().surface.on_color.with_alpha_f32(0.1),
        );
        let mut y = Px::ZERO;
        let mut offset = false;
        let mut gfx = context.gfx.clipped_to(rect);
        while y < rect.size.height {
            let mut x = if offset { checker_size } else { Px::ZERO };
            while x < rect.size.width {
                gfx.draw_shape(shape.translate_by(Point::new(x, y)));
                x += checker_size * 2;
            }
            y += checker_size;
            offset = !offset;
        }
    }
}

/// A widget that selects between completely dark and completely light by
/// utilizing a back-to-white gradient.
#[derive(Debug)]
pub struct ComponentPicker<Component> {
    value: Dynamic<ZeroToOne>,
    visible_rect: Rect<Px>,
    component: Component,
}

impl ComponentPicker<Lightness> {
    /// Returns a new picker that updates `value` when a new lightness is
    /// selected.
    pub fn lightness(value: impl IntoDynamic<ZeroToOne>) -> Self {
        Self::new(value, Lightness)
    }
}

impl ComponentPicker<Red> {
    /// Returns a new picker that updates `value` when a new red is selected.
    pub fn red(value: impl IntoDynamic<u8>) -> Self {
        Self::new(
            value.into_dynamic().linked(
                |value| value.percent_between(&0, &255),
                |percent| 0.lerp(&255, **percent),
            ),
            Red,
        )
    }
}

impl ComponentPicker<Green> {
    /// Returns a new picker that updates `value` when a new green is selected.
    pub fn green(value: impl IntoDynamic<u8>) -> Self {
        Self::new(
            value.into_dynamic().linked(
                |value| value.percent_between(&0, &255),
                |percent| 0.lerp(&255, **percent),
            ),
            Green,
        )
    }
}

impl ComponentPicker<Blue> {
    /// Returns a new picker that updates `value` when a new blue is selected.
    pub fn blue(value: impl IntoDynamic<u8>) -> Self {
        Self::new(
            value.into_dynamic().linked(
                |value| value.percent_between(&0, &255),
                |percent| 0.lerp(&255, **percent),
            ),
            Blue,
        )
    }
}

impl ComponentPicker<Alpha> {
    /// Returns a new picker that updates `value` when a new blue is selected.
    pub fn alpha(value: impl IntoDynamic<u8>, preview_color: impl IntoReadOnly<Color>) -> Self {
        Self::alpha_f32(
            value.into_dynamic().linked(
                |value| value.percent_between(&0, &255),
                |percent| 0.lerp(&255, **percent),
            ),
            preview_color,
        )
    }

    /// Returns a new picker that updates `value` when a new blue is selected.
    pub fn alpha_f32(
        value: impl IntoDynamic<ZeroToOne>,
        preview_color: impl IntoReadOnly<Color>,
    ) -> Self {
        Self::new(
            value,
            Alpha {
                color: preview_color.into_read_only(),
            },
        )
    }
}

impl<Component> ComponentPicker<Component> {
    fn new(value: impl IntoDynamic<ZeroToOne>, component: Component) -> Self {
        Self {
            value: value.into_dynamic(),
            visible_rect: Rect::default(),
            component,
        }
    }

    fn update_from_mouse(&mut self, location: Point<Px>) {
        let relative = (location - self.visible_rect.origin)
            .clamp(Point::ZERO, Point::from(self.visible_rect.size));

        let lightness = relative.x.into_float() / self.visible_rect.size.width.into_float();

        self.value.set(ZeroToOne::new(lightness));
    }
}

impl<Component> Widget for ComponentPicker<Component>
where
    Component: ColorComponent,
{
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let ideal_height = Lp::points(24).into_upx(context.gfx.scale()).ceil();
        Size::new(
            match available_space.width {
                ConstraintLimit::Fill(width) => width,
                ConstraintLimit::SizeToFit(max_width) => max_width.min(ideal_height * 4),
            },
            match available_space.height {
                ConstraintLimit::Fill(height) => height,
                ConstraintLimit::SizeToFit(max_height) => max_height.min(ideal_height),
            },
        )
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        const STEPS: u8 = 10;
        let loupe_size = Lp::mm(3).into_px(context.gfx.scale());
        let size = context.gfx.region().size;

        let outline_color = if context.focused(true) {
            context.get(&HighlightColor)
        } else {
            context.get(&OutlineColor)
        };

        let options = StrokeOptions::px_wide(
            context
                .get(&OutlineWidth)
                .into_px(context.gfx.scale())
                .ceil(),
        )
        .colored(outline_color);
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
        self.component.draw_background(self.visible_rect, context);
        let mut gray = self.component.start_color();
        for step in 0..STEPS {
            let (end_x, end_gray) = if step == STEPS - 1 {
                (bottom_right.x, self.component.end_color())
            } else {
                lightness = ZeroToOne::new(*lightness + step_lightness);
                (x + step_width, self.component.interpolate_color(lightness))
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
        let selected_color = self
            .component
            .loupe_color(value)
            .ok_or_else(|| self.component.interpolate_color(value));
        let mut selected_color = match selected_color {
            Ok(selected_color) => {
                context.gfx.draw_shape(&Shape::filled_round_rect(
                    loupe_rect,
                    CornerRadii::from(loupe_size),
                    selected_color,
                ));
                selected_color
            }
            Err(selected_color) => selected_color,
        };

        let alpha = selected_color.alpha();
        if alpha < 255 {
            let alpha_f32 = alpha.percent_between(&0, &255);
            let surface = context.theme().surface.color;
            selected_color = surface.lerp(&selected_color.with_alpha(255), *alpha_f32);
        }

        let loupe_color =
            selected_color.most_contrasting(&[context.get(&SurfaceColor), context.get(&TextColor)]);

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

        let options =
            StrokeOptions::px_wide(context.get(&OutlineWidth).into_px(context.gfx.scale()))
                .colored(outline_color);
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
