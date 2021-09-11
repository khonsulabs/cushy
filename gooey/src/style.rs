use gooey_core::{
    figures::Figure,
    styles::{
        style_sheet::{Rule, StyleSheet},
        Alignment, BackgroundColor, Border, BorderOptions, Color, ColorPair, ForegroundColor,
        HighlightColor, Padding, Surround, VerticalAlignment,
    },
    PRIMARY_WIDGET_CLASS, ROOT_CLASS, SOLID_WIDGET_CLASS,
};
use gooey_widgets::{
    button::{Button, ButtonColor, ButtonImageSpacing},
    checkbox::Checkbox,
    form,
    input::Input,
    label::Label,
    list::{List, ListAdornmentSpacing},
};

/// The default [`StyleSheet`] for `Gooey`.
#[must_use]
pub fn default_stylesheet() -> StyleSheet {
    stylesheet_for_palette::<()>()
}

/// Creates a stylesheet using the [`Palette`] provided.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn stylesheet_for_palette<P: Palette>() -> StyleSheet {
    StyleSheet::default()
        .with(Rule::default().with_styles(|style| {
            style
                .with(HighlightColor(P::secondary()))
                .with(ForegroundColor(P::foreground()))
        }))
        .with(
            Rule::for_classes(ROOT_CLASS)
                .with_styles(|style| style.with(BackgroundColor(P::background()))),
        )
        .with(
            Rule::for_classes("gooey-navigator-bar")
                .with_styles(|style| style.with(BackgroundColor(P::navigator_background()))),
        )
        .with(
            Rule::for_classes("gooey-navigator-button")
                .with_styles(|style| style.with(ButtonColor(P::navigator_button()))),
        )
        .with(
            Rule::for_classes("gooey-navigator-button")
                .when_hovered()
                .when_not_active()
                .with_styles(|style| style.with(ButtonColor(P::navigator_button().lighten(0.1)))),
        )
        .with(
            Rule::for_classes("gooey-navigator-button")
                .when_active()
                .with_styles(|style| style.with(ButtonColor(P::navigator_button().darken(0.1)))),
        )
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .with_styles(|style| style.with(BackgroundColor(P::control_background()))),
        )
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .when_hovered()
                .when_not_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().lighten(0.2)))
                        .with(BackgroundColor(P::control_background().lighten(0.2)))
                }),
        )
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .when_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().darken(0.1)))
                        .with(BackgroundColor(P::control_background().darken(0.1)))
                }),
        )
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .when_focused()
                .with_styles(|style| {
                    style.with(Border::uniform(BorderOptions::new(1., P::secondary())))
                }),
        )
        .with(
            Rule::for_classes(PRIMARY_WIDGET_CLASS)
                .with_styles(|style| style.with(BackgroundColor(P::primary()))),
        )
        .with(
            Rule::for_classes(PRIMARY_WIDGET_CLASS)
                .when_hovered()
                .when_not_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().lighten(0.2)))
                        .with(BackgroundColor(P::primary().lighten(0.2)))
                }),
        )
        .with(
            Rule::for_classes(PRIMARY_WIDGET_CLASS)
                .when_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().darken(0.1)))
                        .with(BackgroundColor(P::primary().darken(0.1)))
                }),
        )
        .with(
            Rule::for_classes(PRIMARY_WIDGET_CLASS)
                .when_focused()
                .with_styles(|style| {
                    style.with(Border::uniform(BorderOptions::new(1., P::secondary())))
                }),
        )
        .with(Rule::for_widget::<Button>().with_styles(|style| {
            style
                .with(Alignment::Center)
                .with(VerticalAlignment::Center)
                .with(Padding(Surround::from(Some(Figure::new(5.)))))
                .with(ButtonImageSpacing(Figure::new(5.)))
        }))
        .with(
            Rule::for_widget::<Checkbox>()
                .with_styles(|style| style.with(ButtonColor(P::control_background()))),
        )
        .with(
            Rule::for_widget::<Checkbox>()
                .when_hovered()
                .when_not_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().lighten(0.1)))
                        .with(BackgroundColor(P::background().lighten(0.1)))
                }),
        )
        .with(
            Rule::for_widget::<Checkbox>()
                .when_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(P::foreground().darken(0.1)))
                        .with(BackgroundColor(P::background().darken(0.1)))
                }),
        )
        .with(
            Rule::for_widget::<Label>()
                .with_styles(|style| style.with(ForegroundColor(P::foreground()))),
        )
        .with(Rule::for_widget::<Input>().with_styles(|style| {
            style
                .with(BackgroundColor(P::background()))
                .with(Border::uniform(BorderOptions::new(
                    1.,
                    P::control_background(),
                )))
                .with(Padding(Surround::from(Some(Figure::new(5.)))))
        }))
        .with(
            Rule::for_widget::<Input>()
                .when_focused()
                .with_styles(|style| {
                    style.with(Border::uniform(BorderOptions::new(1., P::secondary())))
                }),
        )
        .with(
            Rule::for_widget::<List>()
                .with_styles(|style| style.with(ListAdornmentSpacing(Figure::new(5.)))),
        )
        .with(
            Rule::for_classes(form::LABEL_CLASS)
                .with_styles(|style| style.with(Padding::build().top(10.).bottom(5.).finish())),
        )
}

/// Returns a set of colors that form a palette.
pub trait Palette {
    /// The background color. Used as the window background color, but also used
    /// to derive other neutral background colors.
    fn background() -> ColorPair;
    /// The foreground color. Used as the default text/stroke color, but also
    /// used to derive other neutral foreground colors.
    fn foreground() -> ColorPair;
    /// The primary color of the palette. Used to signify default actions, and
    /// is considered the primary accent color of the interface.
    fn primary() -> ColorPair;
    /// The secondary color of the palette. Used for highlights and information.
    fn secondary() -> ColorPair;

    /// The color to use for the background of solid widgets.
    fn control_background() -> ColorPair {
        Self::background().lighten(0.1)
    }

    /// The color for the [`Navigator`](gooey_widgets::navigator::Navigator) bar's background.
    fn navigator_background() -> ColorPair {
        Self::primary()
    }

    /// The color for buttons within the [`Navigator`](gooey_widgets::navigator::Navigator) bar.
    fn navigator_button() -> ColorPair {
        Self::primary().darken(0.2)
    }
}

impl Palette for () {
    fn background() -> ColorPair {
        ColorPair {
            light_color: Color::WHITE,
            dark_color: Color::BLACK,
        }
    }

    fn foreground() -> ColorPair {
        ColorPair {
            light_color: Color::gray(0.1),
            dark_color: Color::gray(0.9),
        }
    }

    fn primary() -> ColorPair {
        ColorPair::from(Color::new_u8(9, 132, 227, 255))
    }

    fn secondary() -> ColorPair {
        ColorPair::from(Color::new_u8(0, 206, 201, 255))
    }
}
