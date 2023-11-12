//! All style components supported by the built-in widgets.
use std::borrow::Cow;

use kludgine::figures::units::{Lp, Px};
use kludgine::figures::Rect;
use kludgine::Color;

use crate::animation::easings::{EaseInQuadradic, EaseOutQuadradic};
use crate::animation::EasingFunction;
use crate::context::WidgetContext;
use crate::styles::{
    Component, ComponentDefinition, ComponentName, Dimension, Global, NamedComponent,
};

macro_rules! define_components {
    ($($widget:ident { $($(#$doc:tt)* $component:ident($type:ty, $name:expr, $($default:tt)*))* })*) => {$($(
        $(#$doc)*
        #[derive(Clone, Copy, Eq, PartialEq, Debug)]
        pub struct $component;

        const _: () = {
            use $crate::styles::{ComponentDefinition, ComponentName, NamedComponent};
            impl NamedComponent for $component {
                fn name(&self) -> Cow<'_, ComponentName> {
                    Cow::Owned(ComponentName::named::<Button>($name))
                }
            }

            impl ComponentDefinition for $component {
                type ComponentType = $type;

                define_components!($type, $($default)*);
            }
        };

    )*)*};
    ($type:ty, . $($path:tt)*) => {
        define_components!($type, |context| context.theme().$($path)*);
    };
    ($type:ty, |$context:ident| $($expr:tt)*) => {
        fn default_value(&self, $context: &WidgetContext<'_, '_>) -> Color {
            $($expr)*
        }
    };
    ($type:ty, @$path:path) => {
        define_components!($type, |context| context.query_style(&$path));
    };
    ($type:ty, contrasting!($bg:ident, $($fg:ident),+ $(,)?)) => {
        define_components!($type, |context| {
            let styles = context.query_styles(&[&$bg, $(&$fg),*]);
            styles.get(&$bg, context).most_contrasting(&[
                $(styles.get(&$fg, context)),+
            ])
        });
    };
    ($type:ty, $($expr:tt)*) => {
        define_components!($type, |_context| $($expr)*);
    };
}

/// The [`Dimension`] to use as the size to render text.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TextSize;

impl NamedComponent for TextSize {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("text_size"))
    }
}

impl ComponentDefinition for TextSize {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Dimension {
        Dimension::Lp(Lp::points(12))
    }
}

/// The [`Dimension`] to use to space multiple lines of text.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct LineHeight;

impl NamedComponent for LineHeight {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("line_height"))
    }
}

impl ComponentDefinition for LineHeight {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Dimension {
        Dimension::Lp(Lp::points(14))
    }
}

/// The [`Color`] to use when rendering text.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct SurfaceColor;

impl NamedComponent for SurfaceColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("surface_color"))
    }
}

impl ComponentDefinition for SurfaceColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.color
    }
}

/// The [`Color`] to use when rendering text.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TextColor;

impl NamedComponent for TextColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("text_color"))
    }
}

impl ComponentDefinition for TextColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.on_color
    }
}

/// A [`Color`] to be used as a highlight color.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct HighlightColor;

impl NamedComponent for HighlightColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("highlight_color"))
    }
}

impl ComponentDefinition for HighlightColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().primary.color.with_alpha(128)
    }
}

/// Intrinsic, uniform padding for a widget.
///
/// This component is opt-in and does not automatically work for all widgets.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct IntrinsicPadding;

impl NamedComponent for IntrinsicPadding {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("padding"))
    }
}

impl ComponentDefinition for IntrinsicPadding {
    type ComponentType = Dimension;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Dimension {
        Dimension::Lp(Lp::points(5))
    }
}

/// The [`EasingFunction`] to apply to animations that have no inherent
/// directionality.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Easing;

impl NamedComponent for Easing {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("easing"))
    }
}

impl ComponentDefinition for Easing {
    type ComponentType = EasingFunction;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        EasingFunction::from(EaseInQuadradic)
    }
}

/// The [`EasingFunction`] to apply to animations that transition a value from
/// "nothing" to "something". For example, if an widget is animating a color's
/// alpha channel towards opaqueness, it would query for this style component.
/// Otherwise, it would use [`EasingOut`].
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct EasingIn;

impl NamedComponent for EasingIn {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("easing_in"))
    }
}

impl ComponentDefinition for EasingIn {
    type ComponentType = EasingFunction;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        EasingFunction::from(EaseInQuadradic)
    }
}

/// The [`EasingFunction`] to apply to animations that transition a value from
/// "something" to "nothing". For example, if an widget is animating a color's
/// alpha channel towards transparency, it would query for this style component.
/// Otherwise, it would use [`EasingIn`].
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct EasingOut;

impl NamedComponent for EasingOut {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("easing_out"))
    }
}

impl ComponentDefinition for EasingOut {
    type ComponentType = EasingFunction;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        EasingFunction::from(EaseOutQuadradic)
    }
}

/// A 2d ordering configuration.
#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub struct VisualOrder {
    /// The ordering to apply horizontally.
    pub horizontal: HorizontalOrder,
    /// The ordering to apply vertically.
    pub vertical: VerticalOrder,
}

impl VisualOrder {
    /// Returns a right-to-left ordering.
    #[must_use]
    pub const fn right_to_left() -> Self {
        Self {
            horizontal: HorizontalOrder::RightToLeft,
            vertical: VerticalOrder::TopToBottom,
        }
    }

    /// Returns a left-to-right ordering.
    #[must_use]
    pub const fn left_to_right() -> Self {
        Self {
            horizontal: HorizontalOrder::LeftToRight,
            vertical: VerticalOrder::TopToBottom,
        }
    }

    /// Returns the reverse ordering of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        Self {
            horizontal: self.horizontal.rev(),
            vertical: self.vertical.rev(),
        }
    }
}

/// The [`VisualOrder`] strategy to use when laying out content.
#[derive(Debug)]
pub struct LayoutOrder;

impl NamedComponent for LayoutOrder {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("visual_order"))
    }
}

impl ComponentDefinition for LayoutOrder {
    type ComponentType = VisualOrder;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        VisualOrder::left_to_right()
    }
}

impl From<VisualOrder> for Component {
    fn from(value: VisualOrder) -> Self {
        Self::VisualOrder(value)
    }
}

impl TryFrom<Component> for VisualOrder {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::VisualOrder(order) => Ok(order),
            other => Err(other),
        }
    }
}

/// A horizontal direction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HorizontalOrder {
    /// Describes an order starting at the left and proceeding to the right.
    LeftToRight,
    /// Describes an order starting at the right and proceeding to the left.
    RightToLeft,
}

impl HorizontalOrder {
    /// Returns the reverse order of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        match self {
            Self::LeftToRight => Self::RightToLeft,
            Self::RightToLeft => Self::LeftToRight,
        }
    }

    pub(crate) fn sort_key(self, rect: &Rect<Px>) -> Px {
        match self {
            HorizontalOrder::LeftToRight => rect.origin.x,
            HorizontalOrder::RightToLeft => -(rect.origin.x + rect.size.width),
        }
    }
}

/// A vertical direction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum VerticalOrder {
    /// Describes an order starting at the top and proceeding to the bottom.
    TopToBottom,
    /// Describes an order starting at the bottom and proceeding to the top.
    BottomToTop,
}

impl VerticalOrder {
    /// Returns the reverse order of `self`.
    #[must_use]
    pub fn rev(self) -> Self {
        match self {
            Self::TopToBottom => VerticalOrder::BottomToTop,
            Self::BottomToTop => VerticalOrder::TopToBottom,
        }
    }

    pub(crate) fn max_px(self) -> Px {
        match self {
            VerticalOrder::TopToBottom => Px::MAX,
            VerticalOrder::BottomToTop => Px::MIN,
        }
    }

    pub(crate) fn smallest_px(self, a: Px, b: Px) -> Px {
        match self {
            VerticalOrder::TopToBottom => a.min(b),
            VerticalOrder::BottomToTop => b.max(a),
        }
    }
}

/// The set of controls to allow focusing via tab key and initial focus
/// selection.
pub struct AutoFocusableControls;

impl NamedComponent for AutoFocusableControls {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("focus"))
    }
}

impl ComponentDefinition for AutoFocusableControls {
    type ComponentType = FocusableWidgets;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Self::ComponentType {
        FocusableWidgets::default()
    }
}

impl From<FocusableWidgets> for Component {
    fn from(value: FocusableWidgets) -> Self {
        Self::FocusableWidgets(value)
    }
}

impl TryFrom<Component> for FocusableWidgets {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        match value {
            Component::FocusableWidgets(focus) => Ok(focus),
            other => Err(other),
        }
    }
}

/// A configuration option to control which controls should be able to receive
/// focus through keyboard focus handling or initial focus handling.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum FocusableWidgets {
    /// Allow all widgets that can respond to keyboard input to accept focus.
    #[default]
    All,
    /// Only allow widgets that expect textual input to accept focus.
    OnlyTextual,
}

impl FocusableWidgets {
    /// Returns true if all controls should be focusable.
    #[must_use]
    pub const fn is_all(self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns true if only textual should be focusable.
    #[must_use]
    pub const fn is_only_textual(self) -> bool {
        matches!(self, Self::OnlyTextual)
    }
}

/// A [`Color`] to be used as a highlight color.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct WidgetBackground;

impl NamedComponent for WidgetBackground {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("widget_background_color"))
    }
}

impl ComponentDefinition for WidgetBackground {
    type ComponentType = Color;

    fn default_value(&self, _context: &WidgetContext<'_, '_>) -> Color {
        Color::CLEAR_WHITE
    }
}

/// A [`Color`] to be used as an outline color.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct OutlineColor;

impl NamedComponent for OutlineColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("outline_color"))
    }
}

impl ComponentDefinition for OutlineColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.outline
    }
}

/// A [`Color`] to be used as an outline color.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct DisabledOutlineColor;

impl NamedComponent for DisabledOutlineColor {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Global>("disabled_outline_color"))
    }
}

impl ComponentDefinition for DisabledOutlineColor {
    type ComponentType = Color;

    fn default_value(&self, context: &WidgetContext<'_, '_>) -> Color {
        context.theme().surface.outline_variant
    }
}
