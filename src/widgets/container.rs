//! A visual container widget.

use std::ops::Div;

use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    Abs, Angle, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero,
};
use kludgine::shapes::{CornerRadii, PathBuilder, Shape};
use kludgine::Color;

use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::styles::components::{CornerRadius, IntrinsicPadding, Opacity, SurfaceColor};
use crate::styles::{Component, ContainerLevel, Dimension, Edges, RequireInvalidation, Styles};
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{MakeWidget, RootBehavior, Widget, WidgetInstance, WidgetRef};
use crate::ConstraintLimit;

/// A visual container widget, optionally applying padding and a background
/// color.
///
/// # Background Color Selection
///
/// This widget has three different modes for coloring its background:
///
/// - [`ContainerBackground::Auto`]: The background color is automatically
///   selected by using the [next](ContainerLevel::next) level from the next
///   parent container in the hierarchy.
///
///   If the previous container is [`ContainerLevel::Highest`] or the previous
///   parent container uses a color instead of a level,
///   [`ContainerLevel::Lowest`] will be used.
/// - [`ContainerBackground::Color`]: The specified color will be drawn.
/// - [`ContainerBackground::Level`]: The
///   [`SurfaceTheme`](crate::styles::SurfaceTheme) container color associated
///   with the given level will be used.
#[derive(Debug)]
pub struct Container {
    /// The configured background selection.
    pub background: Value<ContainerBackground>,
    /// Padding to surround the contained widget.
    ///
    /// If this is None, a uniform surround of [`IntrinsicPadding`] will be
    /// applied.
    pub padding: Option<Value<Edges<Dimension>>>,
    /// The shadow to apply behind the container's background.
    pub shadow: Value<ContainerShadow>,
    child: WidgetRef,
    applied_background: Option<EffectiveBackground>,
}

/// A strategy of applying a background to a [`Container`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum ContainerBackground {
    /// Automatically select a [`ContainerLevel`] by picking the
    /// [next](ContainerLevel::next) level after the previous parent
    /// [`Container`].
    ///
    /// If no parent container is found or a parent container is found with a
    /// [color](Self::Color) background, [`ContainerLevel::Lowest`] will be
    /// used. See [`Self::Level`] for more information.
    #[default]
    Auto,
    /// Fills the background with the specified color.
    Color(Color),
    /// Applies the [`SurfaceTheme`][st] color
    /// corresponding with the given level.
    ///
    /// | [`ContainerLevel`] | [`SurfaceTheme`][st] property |
    /// |--------------------|-------------------------------|
    /// | [`Lowest`][ll]     | [`lowest_container`][llc]     |
    /// | [`Low`][lo]        | [`low_container`][loc]        |
    /// | [`Low`][mi]        | [`container`][mic]            |
    /// | [`High`][hi]       | [`high_container`][hic]       |
    /// | [`Highest`][hh]    | [`highest_container`][hhc]    |
    ///
    /// [st]: crate::styles::SurfaceTheme
    /// [ll]: ContainerLevel::Lowest
    /// [llc]: crate::styles::SurfaceTheme::lowest_container
    /// [lo]: ContainerLevel::Low
    /// [loc]: crate::styles::SurfaceTheme::low_container
    /// [mi]: ContainerLevel::Mid
    /// [mic]: crate::styles::SurfaceTheme::container
    /// [hi]: ContainerLevel::High
    /// [hic]: crate::styles::SurfaceTheme::high_container
    /// [hh]: ContainerLevel::Highest
    /// [hhc]: crate::styles::SurfaceTheme::highest_container
    Level(ContainerLevel),
}

impl From<ContainerLevel> for ContainerBackground {
    fn from(value: ContainerLevel) -> Self {
        Self::Level(value)
    }
}

impl From<Color> for ContainerBackground {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl Container {
    /// Returns a new container wrapping `child` with default padding and a
    /// background color automatically selected by the theme.
    ///
    /// See [`ContainerBackground::Auto`] for more information about automatic
    /// coloring.
    #[must_use]
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            padding: None,
            applied_background: None,
            background: Value::default(),
            shadow: Value::default(),
            child: WidgetRef::new(child),
        }
    }

    /// Pads the contained widget with `padding`, returning the updated
    /// container.
    #[must_use]
    pub fn pad_by(mut self, padding: impl IntoValue<Edges<Dimension>>) -> Self {
        self.padding = Some(padding.into_value());
        self
    }

    /// Sets this container to render no background color, and then returns the
    /// updated container.
    #[must_use]
    pub fn transparent(mut self) -> Self {
        self.background = Value::Constant(ContainerBackground::Color(Color::CLEAR_WHITE));
        self
    }

    /// Sets this container to use the specific container level, and then
    /// returns the updated container.
    #[must_use]
    pub fn contain_level(mut self, level: impl IntoValue<ContainerLevel>) -> Container {
        self.background = level
            .into_value()
            .map_each(|level| ContainerBackground::from(*level));
        self
    }

    /// Sets this container to render the specified `color` background, and then
    /// returns the updated container.
    #[must_use]
    pub fn background_color(mut self, color: impl IntoValue<Color>) -> Self {
        self.background = color
            .into_value()
            .map_each(|color| ContainerBackground::from(*color));
        self
    }

    /// Renders `shadow` behind the container's background.
    #[must_use]
    pub fn shadow(mut self, shadow: impl IntoValue<ContainerShadow>) -> Self {
        self.shadow = shadow.into_value();
        self
    }

    fn padding(&self, context: &GraphicsContext<'_, '_, '_, '_, '_>) -> Edges<Px> {
        match &self.padding {
            Some(padding) => padding.get(),
            None => Edges::from(context.get(&IntrinsicPadding)),
        }
        .map(|dim| dim.into_px(context.gfx.scale()))
    }

    fn effective_shadow(&self, context: &WidgetContext<'_, '_>) -> ContainerShadow {
        self.shadow.invalidate_when_changed(context);
        self.shadow.get()
    }

    fn effective_background_color(&mut self, context: &WidgetContext<'_, '_>) -> kludgine::Color {
        let background = match self.background.get() {
            ContainerBackground::Color(color) => EffectiveBackground::Color(color),
            ContainerBackground::Level(level) => EffectiveBackground::Level(level),
            ContainerBackground::Auto => {
                EffectiveBackground::Level(match context.get(&CurrentContainerBackground) {
                    EffectiveBackground::Color(_) => ContainerLevel::default(),
                    EffectiveBackground::Level(level) => level.next().unwrap_or_default(),
                })
            }
        };

        if self.applied_background != Some(background) {
            context.attach_styles(Styles::new().with(&CurrentContainerBackground, background));
            self.applied_background = Some(background);
        }

        match background {
            EffectiveBackground::Color(color) => color,
            EffectiveBackground::Level(level) => match level {
                ContainerLevel::Lowest => context.theme().surface.lowest_container,
                ContainerLevel::Low => context.theme().surface.low_container,
                ContainerLevel::Mid => context.theme().surface.container,
                ContainerLevel::High => context.theme().surface.high_container,
                ContainerLevel::Highest => context.theme().surface.highest_container,
            },
        }
    }
}

impl Widget for Container {
    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Container")
            .field("background", &self.background)
            .field("padding", &self.padding)
            .field("shadow", &self.shadow)
            .field("child", &self.child)
            .finish()
    }

    fn full_control_redraw(&self) -> bool {
        true
    }

    #[allow(clippy::too_many_lines)]
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let opacity = context.get(&Opacity);

        let background = self.effective_background_color(context);
        let background = background.with_alpha_f32(background.alpha_f32() * *opacity);
        if background.alpha() > 0 {
            let shadow = self.effective_shadow(context).into_px(context.gfx.scale());

            let child_shadow_offset = shadow.offset.min(Point::ZERO).abs();
            let child_size = context.gfx.region().size - shadow.spread * 2 - shadow.offset.abs();
            let child_area = Rect::new(child_shadow_offset + shadow.spread, child_size);

            let corner_radii = context.get(&CornerRadius).into_px(context.gfx.scale());

            // check if the shadow would be obscured before we try to draw it.
            if child_area.origin != Point::ZERO || child_size != context.gfx.region().size {
                render_shadow(&child_area, corner_radii, &shadow, background, context);
            }

            context.gfx.draw_shape(&Shape::filled_round_rect(
                child_area,
                corner_radii,
                background,
            ));
        }

        let child = self.child.mounted(context);
        context.for_other(&child).redraw();
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let child = self.child.mounted(context);

        let corner_radii = context.get(&CornerRadius).into_upx(context.gfx.scale());

        let max_space = available_space.map(ConstraintLimit::max);
        let min_dimension = max_space.width.min(max_space.height);
        let max_corner_radii = min_dimension / 2;

        let corner_radii = corner_radii.map(|r| r.min(max_corner_radii));

        let mut padding = self.padding(context).into_upx(context.gfx.scale());
        padding.left = padding
            .left
            .max(corner_radii.top_left / std::f32::consts::PI)
            .max(corner_radii.bottom_left / std::f32::consts::PI);
        padding.right = padding
            .right
            .max(corner_radii.top_right / std::f32::consts::PI)
            .max(corner_radii.bottom_right / std::f32::consts::PI);
        padding.top = padding
            .top
            .max(corner_radii.top_right / std::f32::consts::PI)
            .max(corner_radii.top_left / std::f32::consts::PI);
        padding.bottom = padding
            .bottom
            .max(corner_radii.bottom_right / std::f32::consts::PI)
            .max(corner_radii.bottom_left / std::f32::consts::PI);
        let padding_amount = padding.size();

        let shadow = self.effective_shadow(context).into_px(context.gfx.scale());
        let shadow_spread = shadow.spread.into_unsigned();

        let child_shadow_offset_amount = shadow.offset.abs().into_unsigned();
        let child_size = context.for_other(&child).layout(
            available_space - padding_amount - child_shadow_offset_amount - shadow_spread * 2,
        );

        let child_shadow_offset = shadow.offset.min(Point::ZERO).abs().into_unsigned();
        context.set_child_layout(
            &child,
            Rect::new(
                Point::new(padding.left, padding.top) + child_shadow_offset + shadow_spread,
                child_size,
            )
            .into_signed(),
        );

        child_size + padding_amount + child_shadow_offset_amount + shadow_spread * 2
    }

    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_, '_>,
    ) -> Option<(RootBehavior, WidgetInstance)> {
        // TODO adjust for shadow, but we need to potentially merge multiple
        // dimensions into one.
        let mut padding = self
            .padding
            .as_ref()
            .map(|padding| padding.get().into_px(context.kludgine.scale()))
            .unwrap_or_default();
        let shadow = self
            .effective_shadow(context)
            .into_px(context.kludgine.scale());

        if shadow.offset.x >= 0 {
            padding.right += shadow.offset.x;
        } else {
            padding.left += shadow.offset.x.abs();
        }

        if shadow.spread > 0 {
            padding += Edges::from(shadow.spread);
        }

        let behavior = if padding.is_zero() {
            RootBehavior::PassThrough
        } else {
            RootBehavior::Pad(padding.map(Dimension::from))
        };
        Some((behavior, self.child.widget().clone()))
    }
}

#[allow(clippy::too_many_lines)]
fn render_shadow(
    child_area: &Rect<Px>,
    mut corner_radii: CornerRadii<Px>,
    shadow: &ContainerShadow<Px>,
    background: Color,
    context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
) {
    let shadow_color = shadow.color.unwrap_or_else(|| context.theme_pair().shadow);
    let shadow_color =
        shadow_color.with_alpha_f32(shadow_color.alpha_f32() * background.alpha_f32());

    let min_dimension = child_area.size.width.min(child_area.size.height);
    let max_corner_radii = min_dimension / 2;

    corner_radii = corner_radii.map(|r| r.min(max_corner_radii));

    let max_corner = corner_radii
        .top_left
        .max(corner_radii.top_right)
        .max(corner_radii.bottom_left)
        .max(corner_radii.bottom_right);

    let max_blur = min_dimension / 2 - max_corner;
    let blur = shadow.blur_radius.min(max_blur).max(Px::ZERO);
    let gradient_size = shadow.spread + blur;

    if gradient_size > 0 {
        let mut solid_area = Rect::new(Point::squared(gradient_size), child_area.size - blur * 2);
        solid_area.origin += shadow.offset.max(Point::ZERO);

        let transparent = shadow_color.with_alpha(0);
        let solid_left = solid_area.origin.x;
        let solid_right = solid_area.origin.x + solid_area.size.width;
        let solid_top = solid_area.origin.y;
        let solid_bottom = solid_area.origin.y + solid_area.size.height;

        let solid_left_at_top = solid_area.origin.x + corner_radii.top_left;
        let solid_left_at_bottom = solid_area.origin.x + corner_radii.bottom_left;
        let solid_right_at_top =
            solid_area.origin.x + solid_area.size.width - corner_radii.top_right;
        let solid_right_at_bottom =
            solid_area.origin.x + solid_area.size.width - corner_radii.bottom_right;

        let solid_top_at_left = solid_area.origin.y + corner_radii.top_left;
        let solid_bottom_at_left =
            solid_area.origin.y + solid_area.size.height - corner_radii.bottom_left;
        let solid_top_at_right = solid_area.origin.y + corner_radii.top_right;
        let solid_bottom_at_right =
            solid_area.origin.y + solid_area.size.height - corner_radii.bottom_right;

        // Top
        if solid_left_at_top < solid_right_at_top {
            context.gfx.draw_shape(
                &PathBuilder::new((
                    Point::new(solid_left_at_top, solid_top_at_left),
                    shadow_color,
                ))
                .line_to((Point::new(solid_left_at_top, solid_top), shadow_color))
                .line_to((
                    Point::new(solid_left_at_top, solid_top - gradient_size),
                    transparent,
                ))
                .line_to((
                    Point::new(solid_right_at_top, solid_top - gradient_size),
                    transparent,
                ))
                .line_to((Point::new(solid_right_at_top, solid_top), shadow_color))
                .line_to((
                    Point::new(solid_right_at_top, solid_top_at_right),
                    shadow_color,
                ))
                .close()
                .filled(),
            );
        }
        // Right
        context.gfx.draw_shape(
            &PathBuilder::new((Point::new(solid_right, solid_top_at_right), shadow_color))
                .line_to((
                    Point::new(solid_right + gradient_size, solid_top_at_right),
                    transparent,
                ))
                .line_to((
                    Point::new(solid_right + gradient_size, solid_bottom_at_right),
                    transparent,
                ))
                .line_to((Point::new(solid_right, solid_bottom_at_right), shadow_color))
                .close()
                .filled(),
        );
        context.gfx.draw_shape(
            &PathBuilder::new((
                Point::new(solid_right_at_top, solid_top_at_right),
                shadow_color,
            ))
            .line_to((Point::new(solid_right, solid_top_at_right), shadow_color))
            .line_to((Point::new(solid_right, solid_bottom_at_right), shadow_color))
            .line_to((
                Point::new(solid_right_at_bottom, solid_bottom_at_right),
                shadow_color,
            ))
            .close()
            .filled(),
        );

        // Bottom
        context.gfx.draw_shape(
            &PathBuilder::new((
                Point::new(solid_left_at_bottom, solid_bottom_at_left),
                shadow_color,
            ))
            .line_to((Point::new(solid_left_at_bottom, solid_bottom), shadow_color))
            .line_to((
                Point::new(solid_left_at_bottom, solid_bottom + gradient_size),
                transparent,
            ))
            .line_to((
                Point::new(solid_right_at_bottom, solid_bottom + gradient_size),
                transparent,
            ))
            .line_to((
                Point::new(solid_right_at_bottom, solid_bottom),
                shadow_color,
            ))
            .line_to((
                Point::new(solid_right_at_bottom, solid_bottom_at_right),
                shadow_color,
            ))
            .close()
            .filled(),
        );

        // Left
        context.gfx.draw_shape(
            &PathBuilder::new((
                Point::new(solid_left - gradient_size, solid_top_at_left),
                transparent,
            ))
            .line_to((Point::new(solid_left, solid_top_at_left), shadow_color))
            .line_to((Point::new(solid_left, solid_bottom_at_left), shadow_color))
            .line_to((
                Point::new(solid_left - gradient_size, solid_bottom_at_left),
                transparent,
            ))
            .close()
            .filled(),
        );
        context.gfx.draw_shape(
            &PathBuilder::new((Point::new(solid_left, solid_top_at_left), shadow_color))
                .line_to((
                    Point::new(solid_left_at_top, solid_top_at_left),
                    shadow_color,
                ))
                .line_to((
                    Point::new(solid_left_at_bottom, solid_bottom_at_left),
                    shadow_color,
                ))
                .line_to((Point::new(solid_left, solid_bottom_at_left), shadow_color))
                .close()
                .filled(),
        );

        // Top Right
        shadow_arc(
            Point::new(solid_right_at_top, solid_top_at_right),
            corner_radii.top_right,
            gradient_size,
            shadow_color,
            transparent,
            Angle::degrees(270),
            context,
        );

        // Bottom Right
        shadow_arc(
            Point::new(solid_right_at_bottom, solid_bottom_at_right),
            corner_radii.bottom_right,
            gradient_size,
            shadow_color,
            transparent,
            Angle::degrees(0),
            context,
        );

        // Bottom Left
        shadow_arc(
            Point::new(solid_left_at_bottom, solid_bottom_at_left),
            corner_radii.bottom_left,
            gradient_size,
            shadow_color,
            transparent,
            Angle::degrees(90),
            context,
        );

        // Top Left
        shadow_arc(
            Point::new(solid_left_at_top, solid_top_at_left),
            corner_radii.top_left,
            gradient_size,
            shadow_color,
            transparent,
            Angle::degrees(180),
            context,
        );

        // Center
        context.gfx.draw_shape(
            &PathBuilder::new((
                Point::new(solid_left_at_top, solid_top_at_left),
                shadow_color,
            ))
            .line_to((
                Point::new(solid_right_at_top, solid_top_at_right),
                shadow_color,
            ))
            .line_to((
                Point::new(solid_right_at_bottom, solid_bottom_at_right),
                shadow_color,
            ))
            .line_to((
                Point::new(solid_left_at_bottom, solid_bottom_at_left),
                shadow_color,
            ))
            .close()
            .filled(),
        );
    } else {
        context.gfx.draw_shape(&Shape::filled_round_rect(
            Rect::new(shadow.offset.max(Point::ZERO), child_area.size),
            corner_radii,
            shadow_color,
        ));
    }
}

/// Draws a gradiented arc, quantized into sections to compensate for
/// `lyon_geom` having directional fill tesselator. If a single pair of arcs
/// joined by line segements is tesselated, the gradient "leans" in the
/// orientation of `FillOptions::sweep_orientation` and doesn't look properly
/// circular.
fn shadow_arc(
    origin: Point<Px>,
    radius: Px,
    gradient: Px,
    solid_color: Color,
    transparent_color: Color,
    start_angle: Angle,
    context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
) {
    let full_radius = radius + gradient;
    let mut current_outer_arc = rotate_point(
        origin,
        Point::new(origin.x + full_radius, origin.y),
        start_angle,
    );
    let mut current_inner_arc =
        rotate_point(origin, Point::new(origin.x + radius, origin.y), start_angle);
    let mut angle = Angle::degrees(0);

    while angle < Angle::degrees(90) {
        angle += Angle::degrees(5);

        let outer_arc = rotate_point(
            origin,
            Point::new(origin.x + full_radius, origin.y),
            start_angle + angle,
        );
        if outer_arc == current_outer_arc {
            continue;
        }

        let inner_arc = rotate_point(
            origin,
            Point::new(origin.x + radius, origin.y),
            start_angle + angle,
        );

        let mut path = PathBuilder::new((current_inner_arc, solid_color));
        path = path
            .line_to((current_outer_arc, transparent_color))
            .line_to((outer_arc, transparent_color))
            .line_to((inner_arc, solid_color));
        if inner_arc != current_inner_arc {
            path = path.line_to((current_inner_arc, solid_color));
        }
        context.gfx.draw_shape(&path.close().filled());

        if inner_arc != current_inner_arc {
            let mut path = PathBuilder::new((origin, solid_color));
            path = path
                .line_to((current_inner_arc, solid_color))
                .line_to((inner_arc, solid_color))
                .line_to((origin, solid_color));
            context.gfx.draw_shape(&path.close().filled());
        }

        current_outer_arc = outer_arc;
        current_inner_arc = inner_arc;
    }
}

fn rotate_point(origin: Point<Px>, point: Point<Px>, angle: Angle) -> Point<Px> {
    let cos = angle.into_raidans_f().cos();
    let sin = angle.into_raidans_f().sin();
    let d = point - origin;
    origin + Point::new(d.x * cos - d.y * sin, d.y * cos + d.x * sin)
}

/// The selected background configuration of a [`Container`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EffectiveBackground {
    /// The container rendered using the specified level's theme color.
    Level(ContainerLevel),
    /// The container rendered using the specified color.
    Color(Color),
}

impl TryFrom<Component> for EffectiveBackground {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::Color(color) => Ok(EffectiveBackground::Color(color)),
            Component::ContainerLevel(level) => Ok(EffectiveBackground::Level(level)),
            other => Err(other),
        }
    }
}

impl From<EffectiveBackground> for Component {
    fn from(value: EffectiveBackground) -> Self {
        match value {
            EffectiveBackground::Level(level) => Self::ContainerLevel(level),
            EffectiveBackground::Color(color) => Self::Color(color),
        }
    }
}

impl RequireInvalidation for EffectiveBackground {
    fn requires_invalidation(&self) -> bool {
        false
    }
}

define_components! {
    Container {
        /// The container background behind the current widget.
        CurrentContainerBackground(EffectiveBackground, "background", |context| EffectiveBackground::Color(context.get(&SurfaceColor)))
    }
}

/// A shadow for a [`Container`].
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct ContainerShadow<Unit = Dimension> {
    /// The color of the shadow to use for the solid area.
    ///
    /// This color will be faded to transparent if there is any blur on the
    /// shadow.
    pub color: Option<Color>,
    /// The offset of the shadow.
    pub offset: Point<Unit>,
    /// The radius of the blur.
    pub blur_radius: Unit,
    /// An additional amount of space the blur should be expanded across in all
    /// directions. This increases the physical space of the shadow.
    pub spread: Unit,
}

impl<Unit> ContainerShadow<Unit> {
    /// Returns a new shadow that is offset underneath its contents.
    pub fn new(offset: Point<Unit>) -> Self
    where
        Unit: Default,
    {
        Self {
            color: None,
            offset,
            blur_radius: Unit::default(),
            spread: Unit::default(),
        }
    }

    /// Returns a drop shadow placed `distance` below with a combined
    /// blur/spread radius of `blur`.
    pub fn drop(distance: Unit, blur: Unit) -> Self
    where
        Unit: Zero + Div<i32, Output = Unit> + Default + Copy,
    {
        let half_blur = blur / 2;
        Self::new(Point::new(Unit::ZERO, distance))
            .blur_radius(half_blur)
            .spread(half_blur)
    }

    /// Sets the shadow color and returns self.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets the blur radius and returns self.
    #[must_use]
    pub fn blur_radius(mut self, radius: Unit) -> Self {
        self.blur_radius = radius;
        self
    }

    /// Sets the spread radius and returns self.
    #[must_use]
    pub fn spread(mut self, spread: Unit) -> Self {
        self.spread = spread;
        self
    }
}

impl<Unit> ScreenScale for ContainerShadow<Unit>
where
    Unit: ScreenScale<Lp = Lp, Px = Px, UPx = UPx>,
{
    type Lp = ContainerShadow<Lp>;
    type Px = ContainerShadow<Px>;
    type UPx = ContainerShadow<UPx>;

    fn into_px(self, scale: kludgine::figures::Fraction) -> Self::Px {
        ContainerShadow {
            color: self.color,
            offset: self.offset.into_px(scale),
            blur_radius: self.blur_radius.into_px(scale),
            spread: self.spread.into_px(scale),
        }
    }

    fn from_px(px: Self::Px, scale: kludgine::figures::Fraction) -> Self {
        Self {
            color: px.color,
            offset: Point::from_px(px.offset, scale),
            blur_radius: Unit::from_px(px.blur_radius, scale),
            spread: Unit::from_px(px.spread, scale),
        }
    }

    fn into_upx(self, scale: kludgine::figures::Fraction) -> Self::UPx {
        ContainerShadow {
            color: self.color,
            offset: self.offset.into_upx(scale),
            blur_radius: self.blur_radius.into_upx(scale),
            spread: self.spread.into_upx(scale),
        }
    }

    fn from_upx(px: Self::UPx, scale: kludgine::figures::Fraction) -> Self {
        Self {
            color: px.color,
            offset: Point::from_upx(px.offset, scale),
            blur_radius: Unit::from_upx(px.blur_radius, scale),
            spread: Unit::from_upx(px.spread, scale),
        }
    }

    fn into_lp(self, scale: kludgine::figures::Fraction) -> Self::Lp {
        ContainerShadow {
            color: self.color,
            offset: self.offset.into_lp(scale),
            blur_radius: self.blur_radius.into_lp(scale),
            spread: self.spread.into_lp(scale),
        }
    }

    fn from_lp(lp: Self::Lp, scale: kludgine::figures::Fraction) -> Self {
        Self {
            color: lp.color,
            offset: Point::from_lp(lp.offset, scale),
            blur_radius: Unit::from_lp(lp.blur_radius, scale),
            spread: Unit::from_lp(lp.spread, scale),
        }
    }
}

impl From<Px> for ContainerShadow {
    fn from(value: Px) -> Self {
        Self::from(Dimension::from(value))
    }
}

impl From<Lp> for ContainerShadow {
    fn from(value: Lp) -> Self {
        Self::from(Dimension::from(value))
    }
}

impl From<Dimension> for ContainerShadow {
    fn from(spread: Dimension) -> Self {
        Self::default().spread(spread)
    }
}

impl From<Point<Lp>> for ContainerShadow {
    fn from(offset: Point<Lp>) -> Self {
        Self::from(offset.map(Dimension::from))
    }
}

impl From<Point<Px>> for ContainerShadow {
    fn from(offset: Point<Px>) -> Self {
        Self::from(offset.map(Dimension::from))
    }
}

impl From<Point<Dimension>> for ContainerShadow {
    fn from(size: Point<Dimension>) -> Self {
        Self::new(size)
    }
}

impl IntoValue<ContainerShadow> for Dimension {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow::from(self).into_value()
    }
}

impl IntoValue<ContainerShadow> for Point<Px> {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow::from(self).into_value()
    }
}

impl IntoValue<ContainerShadow> for Point<Lp> {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow::from(self).into_value()
    }
}

impl IntoValue<ContainerShadow> for Point<Dimension> {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow::from(self).into_value()
    }
}

impl IntoValue<ContainerShadow> for ContainerShadow<Px> {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow {
            color: self.color,
            offset: self.offset.map(Dimension::from),
            blur_radius: Dimension::from(self.blur_radius),
            spread: Dimension::from(self.spread),
        }
        .into_value()
    }
}

impl From<ContainerShadow<Lp>> for ContainerShadow {
    fn from(value: ContainerShadow<Lp>) -> Self {
        ContainerShadow {
            color: value.color,
            offset: value.offset.map(Dimension::from),
            blur_radius: Dimension::from(value.blur_radius),
            spread: Dimension::from(value.spread),
        }
    }
}

impl From<ContainerShadow<Px>> for ContainerShadow {
    fn from(value: ContainerShadow<Px>) -> Self {
        ContainerShadow {
            color: value.color,
            offset: value.offset.map(Dimension::from),
            blur_radius: Dimension::from(value.blur_radius),
            spread: Dimension::from(value.spread),
        }
    }
}

impl IntoValue<ContainerShadow> for ContainerShadow<Lp> {
    fn into_value(self) -> Value<ContainerShadow> {
        ContainerShadow::<Dimension>::from(self).into_value()
    }
}

impl IntoValue<ContainerShadow> for Dynamic<ContainerShadow<Px>> {
    fn into_value(self) -> Value<ContainerShadow> {
        Value::Dynamic(self.map_each_cloned(ContainerShadow::<Dimension>::from))
    }
}

impl IntoValue<ContainerShadow> for Dynamic<ContainerShadow<Lp>> {
    fn into_value(self) -> Value<ContainerShadow> {
        Value::Dynamic(self.map_each_cloned(ContainerShadow::<Dimension>::from))
    }
}
