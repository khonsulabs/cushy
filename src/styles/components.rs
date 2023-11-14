//! All style components supported by the built-in widgets.

use kludgine::figures::units::Lp;
use kludgine::Color;

use crate::animation::easings::{EaseInOutQuadradic, EaseInQuadradic, EaseOutQuadradic};
use crate::animation::EasingFunction;
use crate::styles::{Dimension, FocusableWidgets, VisualOrder};

/// Defines a set of style components for Gooey.
///
/// These macros implement [`NamedComponent`](crate::styles::NamedComponent) and
/// [`ComponentDefinition`](crate::styles::ComponentDefinition) for each entry
/// defined. The syntax is:
///
/// ```rust
/// use gooey::define_components;
/// use gooey::styles::Dimension;
/// use gooey::styles::components::{SurfaceColor, TextColor};
/// use gooey::kludgine::Color;
///
/// define_components! {
///     GroupName {
///         /// This is the documentation for example component. It has a default value of `Dimension::ZERO`.
///         ExampleComponent(Dimension, "example_component", Dimension::ZERO)
///         /// This component whose default value is a color from the current theme.
///         ThemedComponent(Color, "themed_component", .primary.color)
///         /// This component is a color whose default value is the currently defined `TextColor`.
///         DependentComponent(Color, "dependent_component", @TextColor)
///         /// This component defaults to picking a contrasting color between `TextColor` and `SurfaceColor`
///         ContrastingColor(Color, "contrasting_color", contrasting!(ThemedComponent, TextColor, SurfaceColor))
///         /// This component shows how to use a closure for nearly infinite flexibility in computing the default value.
///         ClosureDefaultComponent(Color, "closure_component", |context| context.get(&TextColor))
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_components {
    ($($widget:ident { $($(#$doc:tt)* $component:ident($type:ty, $name:expr, $($default:tt)*))* })*) => {$($(
        $(#$doc)*
        #[derive(Clone, Copy, Eq, PartialEq, Debug)]
        pub struct $component;

        const _: () = {
            use $crate::styles::{ComponentDefinition, ComponentName, NamedComponent};
            use $crate::context::WidgetContext;
            use $crate::Lazy;
            use ::std::borrow::Cow;

            impl NamedComponent for $component {
                fn name(&self) -> Cow<'_, ComponentName> {
                    static NAME: Lazy<ComponentName> = Lazy::new(|| ComponentName::new(stringify!($widget), $name));
                    Cow::Borrowed(&*NAME)
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
        fn default_value(&self, $context: &WidgetContext<'_, '_>) -> $type {
            $($expr)*
        }
    };
    ($type:ty, @$path:path) => {
        define_components!($type, |context| context.get(&$path));
    };
    ($type:ty, contrasting!($bg:ident, $($fg:ident),+ $(,)?)) => {
        define_components!($type, |context| {
            use $crate::styles::ColorExt;
            context.get(&$bg).most_contrasting(&[
                $(context.get(&$fg)),+
            ])
        });
    };
    ($type:ty, $($expr:tt)*) => {
        define_components!($type, |_context| $($expr)*);
    };
}

define_components! {
    Global {
        /// The [`Dimension`] to use as the size to render text.
        TextSize(Dimension, "text_size", Dimension::Lp(Lp::points(12)))
        /// The [`Dimension`] to use to space multiple lines of text.
        LineHeight(Dimension,"line_height",Dimension::Lp(Lp::points(14)))
        /// The [`Color`] of the surface for the user interface to draw upon.
        SurfaceColor(Color, "surface_color", .surface.color)
        /// The [`Color`] to use when rendering text.
        TextColor(Color, "text_color", .surface.on_color)
        /// A [`Color`] to be used as a highlight color.
        HighlightColor(Color,"highlight_color",.primary.color.with_alpha(128))
        /// Intrinsic, uniform padding for a widget.
        ///
        /// This component is opt-in and does not automatically work for all widgets.
        IntrinsicPadding(Dimension, "padding", Dimension::Lp(Lp::points(5)))
        /// The [`EasingFunction`] to apply to animations that have no inherent
        /// directionality.
        Easing(EasingFunction, "Easing", EasingFunction::from(EaseInOutQuadradic))
        /// The [`EasingFunction`] to apply to animations that transition a value from
        /// "nothing" to "something". For example, if an widget is animating a color's
        /// alpha channel towards opaqueness, it would query for this style component.
        /// Otherwise, it would use [`EasingOut`].
        EasingIn(EasingFunction, "easing_out", EasingFunction::from(EaseInQuadradic))
        /// The [`EasingFunction`] to apply to animations that transition a value from
        /// "something" to "nothing". For example, if an widget is animating a color's
        /// alpha channel towards transparency, it would query for this style component.
        /// Otherwise, it would use [`EasingIn`].
        EasingOut(EasingFunction, "easing_out", EasingFunction::from(EaseOutQuadradic))
        /// The [`VisualOrder`] strategy to use when laying out content.
        LayoutOrder(VisualOrder, "visual_order", VisualOrder::left_to_right())
        /// The set of controls to allow focusing via tab key and initial focus
        /// selection.
        AutoFocusableControls(FocusableWidgets, "focus", FocusableWidgets::default())
        /// A [`Color`] to be used as the background color of a widget.
        WidgetBackground(Color, "widget_backgrond_color", Color::CLEAR_WHITE)
        /// A [`Color`] to be used as an outline color.
        OutlineColor(Color, "outline_color", .surface.outline)
        /// A [`Color`] to be used as an outline color.
        DisabledOutlineColor(Color, "disabled_outline_color", .surface.outline_variant)
        /// A [`Color`] to be used as a background color for widgets that render an
        /// opaque background.
        OpaqueWidgetColor(Color, "opaque_color", .surface.opaque_widget)
    }
}
