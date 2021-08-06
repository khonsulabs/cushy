use gooey_core::{
    euclid::Length,
    styles::{
        style_sheet::{Rule, StyleSheet},
        Alignment, BackgroundColor, Color, ColorPair, ForegroundColor, Padding, Surround,
        VerticalAlignment,
    },
    ROOT_CLASS, SOLID_WIDGET_CLASS,
};
use gooey_widgets::{
    button::{Button, ButtonColor},
    checkbox::Checkbox,
    label::Label,
};

/// The default [`StyleSheet`] for `Gooey`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn default_stylesheet() -> StyleSheet {
    StyleSheet::default()
        .with(Rule::for_classes(ROOT_CLASS).with_styles(|style| {
            style.with(BackgroundColor(ColorPair {
                light_color: Color::WHITE,
                dark_color: Color::BLACK,
            }))
        }))
        .with(
            Rule::for_classes("gooey-navigator-bar").with_styles(|style| {
                style.with(BackgroundColor(ColorPair {
                    light_color: Color::gray(0.9),
                    dark_color: Color::gray(0.1),
                }))
            }),
        )
        .with(Rule::for_classes(SOLID_WIDGET_CLASS).with_styles(|style| {
            style
                .with(ForegroundColor(ColorPair {
                    light_color: Color::gray(0.1),
                    dark_color: Color::gray(0.9),
                }))
                .with(BackgroundColor(ColorPair {
                    light_color: Color::gray(0.6),
                    dark_color: Color::gray(0.2),
                }))
        }))
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .when_hovered()
                .when_not_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(ColorPair {
                            light_color: Color::gray(0.2),
                            dark_color: Color::WHITE,
                        }))
                        .with(BackgroundColor(ColorPair {
                            light_color: Color::gray(0.7),
                            dark_color: Color::gray(0.3),
                        }))
                }),
        )
        .with(
            Rule::for_classes(SOLID_WIDGET_CLASS)
                .when_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(ColorPair {
                            light_color: Color::BLACK,
                            dark_color: Color::gray(0.8),
                        }))
                        .with(BackgroundColor(ColorPair {
                            light_color: Color::gray(0.5),
                            dark_color: Color::gray(0.1),
                        }))
                }),
        )
        .with(Rule::for_widget::<Button>().with_styles(|style| {
            style
                .with(Alignment::Center)
                .with(VerticalAlignment::Center)
                .with(Padding(Surround::from(Some(Length::new(5.)))))
        }))
        .with(Rule::for_widget::<Checkbox>().with_styles(|style| {
            style
                .with(ForegroundColor(ColorPair {
                    light_color: Color::gray(0.1),
                    dark_color: Color::gray(0.9),
                }))
                .with(ButtonColor(ColorPair {
                    light_color: Color::gray(0.6),
                    dark_color: Color::gray(0.2),
                }))
        }))
        .with(
            Rule::for_widget::<Checkbox>()
                .when_hovered()
                .when_not_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(ColorPair {
                            light_color: Color::gray(0.2),
                            dark_color: Color::WHITE,
                        }))
                        .with(ButtonColor(ColorPair {
                            light_color: Color::gray(0.7),
                            dark_color: Color::gray(0.3),
                        }))
                }),
        )
        .with(
            Rule::for_widget::<Checkbox>()
                .when_active()
                .with_styles(|style| {
                    style
                        .with(ForegroundColor(ColorPair {
                            light_color: Color::BLACK,
                            dark_color: Color::gray(0.8),
                        }))
                        .with(ButtonColor(ColorPair {
                            light_color: Color::gray(0.5),
                            dark_color: Color::gray(0.1),
                        }))
                }),
        )
        .with(Rule::for_widget::<Label>().with_styles(|style| {
            style.with(ForegroundColor(ColorPair {
                light_color: Color::gray(0.1),
                dark_color: Color::gray(0.9),
            }))
        }))
}
