//! A visual container widget.

use kludgine::figures::units::Px;
use kludgine::figures::{IntoUnsigned, Point, Rect, ScreenScale, Size};
use kludgine::Color;

use crate::context::{GraphicsContext, LayoutContext, WidgetContext};
use crate::styles::components::{IntrinsicPadding, SurfaceColor};
use crate::styles::{Component, ContainerLevel, Dimension, Edges, RequireInvalidation, Styles};
use crate::value::{IntoValue, Value};
use crate::widget::{MakeWidget, WidgetRef, WrappedLayout, WrapperWidget};
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
    child: WidgetRef,
    effective_background: Option<EffectiveBackground>,
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
            effective_background: None,
            background: Value::default(),
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

    fn padding(&self, context: &GraphicsContext<'_, '_, '_, '_, '_>) -> Edges<Px> {
        match &self.padding {
            Some(padding) => padding.get(),
            None => Edges::from(context.get(&IntrinsicPadding)),
        }
        .map(|dim| dim.into_px(context.gfx.scale()))
    }
}

impl WrapperWidget for Container {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn background_color(&mut self, context: &WidgetContext<'_, '_>) -> Option<kludgine::Color> {
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

        if self.effective_background != Some(background) {
            context.attach_styles(Styles::new().with(&CurrentContainerBackground, background));
            self.effective_background = Some(background);
        }

        Some(match background {
            EffectiveBackground::Color(color) => color,
            EffectiveBackground::Level(level) => match level {
                ContainerLevel::Lowest => context.theme().surface.lowest_container,
                ContainerLevel::Low => context.theme().surface.low_container,
                ContainerLevel::Mid => context.theme().surface.container,
                ContainerLevel::High => context.theme().surface.high_container,
                ContainerLevel::Highest => context.theme().surface.highest_container,
            },
        })
    }

    fn adjust_child_constraint(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        let padding_amount = self.padding(context).size().into_upx(context.gfx.scale());
        Size::new(
            available_space.width - padding_amount.width,
            available_space.height - padding_amount.height,
        )
    }

    fn position_child(
        &mut self,
        size: Size<Px>,
        _available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> WrappedLayout {
        let padding = self.padding(context);
        let padded = size + padding.size();

        WrappedLayout {
            child: Rect::new(Point::new(padding.left, padding.top), size),
            size: padded.into_unsigned(),
        }
    }
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
