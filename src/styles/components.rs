//! All style components supported by the built-in widgets.

use figures::units::Lp;
use kludgine::cosmic_text::{FamilyOwned, Style, Weight};
use kludgine::shapes::CornerRadii;
use kludgine::Color;

use crate::animation::easings::{EaseInOutQuadradic, EaseInQuadradic, EaseOutQuadradic};
use crate::animation::{EasingFunction, ZeroToOne};
use crate::styles::{Dimension, FocusableWidgets, FontFamilyList, VisualOrder};

/// Defines a set of style components for Cushy.
///
/// These macros implement [`NamedComponent`](crate::styles::NamedComponent) and
/// [`ComponentDefinition`](crate::styles::ComponentDefinition) for each entry
/// defined. The syntax is:
///
/// ```rust
/// use cushy::define_components;
/// use cushy::styles::Dimension;
/// use cushy::styles::components::{SurfaceColor, TextColor};
/// use cushy::kludgine::Color;
/// use cushy::figures::Zero;
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
    ($($widget:ident { $($(#$doc:tt)* $component:ident($type:ty, $name:expr $(, $($default:tt)*)?))* })*) => {$($(
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

                define_components!($type, $($($default)*)?);
            }

            define_components!(default $component $type, $($($default)*)?);
        };

    )*)*};
    ($type:ty, . $($path:tt)*) => {
        define_components!($type, |context| context.theme().$($path)*);
    };
    ($type:ty, |$context:ident| $($expr:tt)*) => {
        fn default_value(&self, $context: &WidgetContext<'_>) -> $type {
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
    ($type:ty, ) => {
        define_components!($type, |_context| <$type>::default());
    };
    ($type:ty, $($expr:tt)+) => {
        define_components!($type, |_context| $($expr)*);
    };
    (default $component:ident $type:ty, . $($path:tt)*) => {

    };
    (default $component:ident $type:ty, |$context:ident| $($expr:tt)*) => {
    };
    (default $component:ident $type:ty, @$path:path) => {
    };
    (default $component:ident $type:ty, contrasting!($bg:ident, $($fg:ident),+ $(,)?)) => {
    };
    (default $component:ident $type:ty, ) => {
        impl $crate::styles::ContextFreeComponent for $component {
            fn default(&self) -> Self::ComponentType {
                <$type>::default()
            }
        }
    };
    (default $component:ident $type:ty, $($expr:tt)+) => {
        impl $crate::styles::ContextFreeComponent for $component {
            fn default(&self) -> Self::ComponentType {
                $($expr)*
            }
        }
    };
}

define_components! {
    Global {
        /// The [`Dimension`] to use as the size to render text.
        TextSize(Dimension, "text_size", @BaseTextSize)
        /// The [`Dimension`] to use to space multiple lines of text.
        LineHeight(Dimension,"line_height", @BaseLineHeight)

        /// The base [`Dimension`] to use as the normal text size. Unless
        /// overridden, all other sizes for built-in widgets will be based on
        /// this dimension.
        BaseTextSize(Dimension, "base_text_size", Dimension::Lp(Lp::points(12)))
        /// The base [`Dimension`] to use to space multiple lines of text.
        /// Unless overridden, all other sizes for built-in widgets will be
        /// based on this dimension.
        BaseLineHeight(Dimension,"base_line_height", Dimension::Lp(Lp::points(16)))
        /// The largest text size on a series of 8 steps.
        TextSize8(Dimension, "text_size_8", |context| context.get(&BaseTextSize) * 2.5)
        /// The second-largest text size on a series of 8 steps.
        TextSize7(Dimension, "text_size_7", |context| context.get(&BaseTextSize) * 2.25)
        /// The third-largest text size on a series of 8 steps.
        TextSize6(Dimension, "text_size_6", |context| context.get(&BaseTextSize) * 2.0)
        /// The fourth-largest text size on a series of 8 steps.
        TextSize5(Dimension, "text_size_5", |context| context.get(&BaseTextSize) * 1.5)
        /// The fifth-largest text size on a series of 8 steps.
        TextSize4(Dimension, "text_size_4", |context| context.get(&BaseTextSize) * 1.25)
        /// The base text size on a series of 8 steps.
        TextSize3(Dimension, "text_size_3", @BaseTextSize)
        /// The second-smallest text size on a series of 8 steps.
        TextSize2(Dimension, "text_size_2", |context| context.get(&BaseTextSize) * 0.75)
        /// The smallest text size on a series of 8 steps.
        TextSize1(Dimension, "text_size_1", |context| context.get(&BaseTextSize) * 0.5)

        /// The largest line height on a series of 8 steps.
        LineHeight8(Dimension, "line_height_8", |context| context.get(&BaseLineHeight) * 2.5)
        /// The second-largest line height on a series of 8 steps.
        LineHeight7(Dimension, "line_height_7", |context| context.get(&BaseLineHeight) * 2.25)
        /// The third-largest line height on a series of 8 steps.
        LineHeight6(Dimension, "line_height_6", |context| context.get(&BaseLineHeight) * 2.0)
        /// The fourth-largest line height on a series of 8 steps.
        LineHeight5(Dimension, "line_height_5", |context| context.get(&BaseLineHeight) * 1.5)
        /// The fifth-largest line height on a series of 8 steps.
        LineHeight4(Dimension, "line_height_4", |context| context.get(&BaseLineHeight) * 1.25)
        /// The base line height on a series of 8 steps.
        LineHeight3(Dimension, "line_height_4", @BaseLineHeight)
        /// The second-smallest line height on a series of 8 steps.
        LineHeight2(Dimension, "line_height_2", |context| context.get(&BaseLineHeight) * 0.75)
        /// The smallest line height on a series of 8 steps.
        LineHeight1(Dimension, "line_height_1", |context| context.get(&BaseLineHeight) * 0.675)

        /// The [`Color`] of the surface for the user interface to draw upon.
        SurfaceColor(Color, "surface_color", .surface.color)
        /// The [`Color`] to use when rendering text.
        TextColor(Color, "text_color", .surface.on_color)
        /// The [`Color`] to use when rendering text in a more subdued tone.
        TextColorVariant(Color, "text_color_variant", .surface.on_color_variant)
        /// A [`Color`] to be used as a highlight color.
        HighlightColor(Color,"highlight_color", .primary.color.with_alpha(128))
        /// A [`Color`] to be used as to indicate keyboard focus.
        FocusColor(Color,"focus_color", @HighlightColor)
        /// The width of outlines drawn around widgets.
        OutlineWidth(Dimension,"outline_width", Dimension::Lp(Lp::points(1)))
        /// The primary color from the current theme.
        PrimaryColor(Color, "primary_color", .primary.color)
        /// The secondary color from the current theme.
        SecondaryColor(Color, "secondary_color", .secondary.color)
        /// The tertiary color from the current theme.
        TertiaryColor(Color, "tertiary_color", .tertiary.color)
        /// The error color from the current theme.
        ErrorColor(Color, "error_color", .error.color)
        /// The foreground color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default).
        DefaultForegroundColor(Color, "default_foreground_color", .primary.on_color)
        /// The background color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default).
        DefaultBackgroundColor(Color, "default_background_color", .primary.color)
        /// The foreground color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is hovered by
        /// the cursor.
        DefaultHoveredForegroundColor(Color, "default_hovered_foreground_color", @DefaultForegroundColor)
        /// The background color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is hovered by
        /// the cursor.
        DefaultHoveredBackgroundColor(Color, "default_hovered_background_color", .primary.color_bright)
        /// The foreground color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is activated.
        DefaultActiveForegroundColor(Color, "default_active_foreground_color", .primary.on_color)
        /// The background color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is activated.
        DefaultActiveBackgroundColor(Color, "default_active_background_color", .primary.color_dim)
        /// The foreground color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is disabled.
        DefaultDisabledForegroundColor(Color, "default_disabled_foreground_color", .primary.on_color)
        /// The background color to use when drawing a [default
        /// widget](crate::widget::MakeWidget::into_default) that is disabled.
        DefaultDisabledBackgroundColor(Color, "default_disabled_background_color", .primary.color_dim)
        /// Intrinsic, uniform padding for a widget.
        ///
        /// This component is opt-in and does not automatically work for all widgets.
        IntrinsicPadding(Dimension, "padding", Dimension::Lp(Lp::points(6)))
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
        AutoFocusableControls(FocusableWidgets, "focus")
        /// A [`Color`] to be used as the background color of a widget.
        WidgetBackground(Color, "widget_backgrond_color", Color::CLEAR_WHITE)
        /// A [`Color`] to be used to accent a widget.
        WidgetAccentColor(Color, "widget_accent_color", .primary.color)
        /// A [`Color`] to be used to accent a disabled widget.
        DisabledWidgetAccentColor(Color, "disabled_widget_accent_color", .primary.color_dim)
        /// A [`Color`] to be used as an outline color.
        OutlineColor(Color, "outline_color", .surface.outline)
        /// A [`Color`] to be used as an outline color.
        DisabledOutlineColor(Color, "disabled_outline_color", .surface.outline_variant)
        /// A [`Color`] to be used as a background color for widgets that render an
        /// opaque background.
        OpaqueWidgetColor(Color, "opaque_color", .surface.opaque_widget)
        /// A [`Color`] to be use for the transparent surface behind an overlay.
        ScrimColor(Color, "scrim_color", |context| context.theme_pair().scrim.with_alpha(70))
        /// A set of radius descriptions for how much roundness to apply to the
        /// shapes of widgets.
        CornerRadius(CornerRadii<Dimension>, "corner_radius", CornerRadii::from(Dimension::Lp(Lp::points(6))))
        /// The font family to render text using.
        FontFamily(FontFamilyList, "font_family", FontFamilyList::from(FamilyOwned::SansSerif))
        /// The font (boldness) weight to apply to text rendering.
        FontWeight(Weight, "font_weight", Weight::NORMAL)
        /// The font style to apply to text rendering.
        FontStyle(Style, "font_style", Style::Normal)

        /// The default [`Weight`] to apply to headings.
        HeadingWeight(Weight, "heading_weight", Weight::BOLD)
        /// The [`Weight`] to apply to h1 headings.
        Heading1Weight(Weight, "heading_weight_1", @HeadingWeight)
        /// The [`Weight`] to apply to h2 headings.
        Heading2Weight(Weight, "heading_weight_2", @HeadingWeight)
        /// The [`Weight`] to apply to h3 headings.
        Heading3Weight(Weight, "heading_weight_3", @HeadingWeight)
        /// The [`Weight`] to apply to h4 headings.
        Heading4Weight(Weight, "heading_weight_4", @HeadingWeight)
        /// The [`Weight`] to apply to h5 headings.
        Heading5Weight(Weight, "heading_weight_5", @HeadingWeight)
        /// The [`Weight`] to apply to h6 headings.
        Heading6Weight(Weight, "heading_weight_6", @HeadingWeight)

        /// The default [`Style`] to apply to headings.
        HeadingStyle(Style, "heading_style", Style::Normal)
        /// The [`Style`] to apply to h1 headings.
        Heading1Style(Style, "heading_style_1", @HeadingStyle)
        /// The [`Style`] to apply to h2 headings.
        Heading2Style(Style, "heading_style_2", @HeadingStyle)
        /// The [`Style`] to apply to h3 headings.
        Heading3Style(Style, "heading_style_3", @HeadingStyle)
        /// The [`Style`] to apply to h4 headings.
        Heading4Style(Style, "heading_style_4", @HeadingStyle)
        /// The [`Style`] to apply to h5 headings.
        Heading5Style(Style, "heading_style_5", @HeadingStyle)
        /// The [`Style`] to apply to h6 headings.
        Heading6Style(Style, "heading_style_6", @HeadingStyle)

        /// The default [`FontFamilyList`] to apply to headings.
        HeadingFontFamily(FontFamilyList, "heading_font_family", FontFamilyList::from(FamilyOwned::SansSerif))
        /// The [`FontFamilyList`] to apply to h1 headings.
        Heading1FontFamily(FontFamilyList, "heading_font_family_1", @HeadingFontFamily)
        /// The [`FontFamilyList`] to apply to h2 headings.
        Heading2FontFamily(FontFamilyList, "heading_font_family_2", @HeadingFontFamily)
        /// The [`FontFamilyList`] to apply to h3 headings.
        Heading3FontFamily(FontFamilyList, "heading_font_family_3", @HeadingFontFamily)
        /// The [`FontFamilyList`] to apply to h4 headings.
        Heading4FontFamily(FontFamilyList, "heading_font_family_4", @HeadingFontFamily)
        /// The [`FontFamilyList`] to apply to h5 headings.
        Heading5FontFamily(FontFamilyList, "heading_font_family_5", @HeadingFontFamily)
        /// The [`FontFamilyList`] to apply to h6 headings.
        Heading6FontFamily(FontFamilyList, "heading_font_family_6", @HeadingFontFamily)

        /// The opaqueness of drawing calls
        Opacity(ZeroToOne, "opacity", ZeroToOne::ONE)
    }
}
