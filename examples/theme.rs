use gooey::animation::ZeroToOne;
use gooey::styles::components::{TextColor, WidgetBackground};
use gooey::styles::{ColorSource, ColorTheme, FixedTheme, SurfaceTheme, Theme, ThemePair};
use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::{Label, Scroll, Slider, Stack, Themed};
use gooey::window::ThemeMode;
use gooey::Run;
use kludgine::Color;

const PRIMARY_HUE: f32 = 240.;
const SECONDARY_HUE: f32 = 0.;
const TERTIARY_HUE: f32 = 330.;
const ERROR_HUE: f32 = 30.;

fn main() -> gooey::Result {
    let (primary, primary_editor) = color_editor(PRIMARY_HUE, 0.8, "Primary");
    let (secondary, secondary_editor) = color_editor(SECONDARY_HUE, 0.3, "Secondary");
    let (tertiary, tertiary_editor) = color_editor(TERTIARY_HUE, 0.3, "Tertiary");
    let (error, error_editor) = color_editor(ERROR_HUE, 0.8, "Error");
    let (neutral, neutral_editor) = color_editor(PRIMARY_HUE, 0.001, "Neutral");
    let (neutral_variant, neutral_variant_editor) =
        color_editor(PRIMARY_HUE, 0.001, "Neutral Variant");
    let (theme_mode, theme_switcher) = dark_mode_slider();

    let default_theme = (
        &primary,
        &secondary,
        &tertiary,
        &error,
        &neutral,
        &neutral_variant,
    )
        .map_each(
            |(primary, secondary, tertiary, error, neutral, neutral_variant)| {
                ThemePair::from_sources(
                    *primary,
                    *secondary,
                    *tertiary,
                    *error,
                    *neutral,
                    *neutral_variant,
                )
            },
        );

    Themed::new(
        default_theme.clone(),
        Stack::columns(
            Scroll::vertical(Stack::rows(
                theme_switcher
                    .and(primary_editor)
                    .and(secondary_editor)
                    .and(tertiary_editor)
                    .and(error_editor)
                    .and(neutral_editor)
                    .and(neutral_variant_editor),
            ))
            .and(theme(default_theme.map_each(|theme| theme.dark), "Dark"))
            .and(theme(default_theme.map_each(|theme| theme.light), "Light"))
            .and(fixed_themes(
                default_theme.map_each(|theme| theme.primary_fixed),
                default_theme.map_each(|theme| theme.secondary_fixed),
                default_theme.map_each(|theme| theme.tertiary_fixed),
            )),
        ),
    )
    .expand()
    .into_window()
    .with_theme_mode(theme_mode)
    .run()
}

fn dark_mode_slider() -> (Dynamic<ThemeMode>, impl MakeWidget) {
    let on_off = Dynamic::new(true);
    let theme_mode = on_off.map_each(|dark| {
        if *dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    });

    (
        theme_mode,
        Stack::rows(Label::new("Theme Mode").and(Slider::<bool>::from_value(on_off))),
    )
}

fn color_editor(
    initial_hue: f32,
    initial_saturation: impl Into<ZeroToOne>,
    label: &str,
) -> (Dynamic<ColorSource>, impl MakeWidget) {
    let hue = Dynamic::new(initial_hue);
    let hue_text = hue.map_each(|hue| hue.to_string());
    let saturation = Dynamic::new(initial_saturation.into());
    let saturation_text = saturation.map_each(|saturation| saturation.to_string());

    let color =
        (&hue, &saturation).map_each(|(hue, saturation)| ColorSource::new(*hue, *saturation));

    (
        color,
        Stack::rows(
            Label::new(label)
                .and(Slider::<f32>::new(hue, 0., 360.))
                .and(Label::new(hue_text))
                .and(Slider::<ZeroToOne>::from_value(saturation))
                .and(Label::new(saturation_text)),
        ),
    )
}

fn fixed_themes(
    primary: Dynamic<FixedTheme>,
    secondary: Dynamic<FixedTheme>,
    tertiary: Dynamic<FixedTheme>,
) -> impl MakeWidget {
    Stack::rows(
        Label::new("Fixed")
            .and(fixed_theme(primary, "Primary"))
            .and(fixed_theme(secondary, "Secondary"))
            .and(fixed_theme(tertiary, "Tertiary")),
    )
    .expand()
}

fn fixed_theme(theme: Dynamic<FixedTheme>, label: &str) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let on_color = theme.map_each(|theme| theme.on_color);
    Stack::columns(
        swatch(color.clone(), &format!("{label} Fixed"), on_color.clone())
            .and(swatch(
                theme.map_each(|theme| theme.dim_color),
                &format!("Dim {label}"),
                on_color.clone(),
            ))
            .and(swatch(
                on_color.clone(),
                &format!("On {label} Fixed"),
                color.clone(),
            ))
            .and(swatch(
                theme.map_each(|theme| theme.on_color_variant),
                &format!("Variant On {label} Fixed"),
                color,
            )),
    )
    .expand()
}

fn theme(theme: Dynamic<Theme>, label: &str) -> impl MakeWidget {
    Stack::rows(
        Label::new(label)
            .and(
                Stack::columns(
                    color_theme(theme.map_each(|theme| theme.primary), "Primary")
                        .and(color_theme(
                            theme.map_each(|theme| theme.secondary),
                            "Secondary",
                        ))
                        .and(color_theme(
                            theme.map_each(|theme| theme.tertiary),
                            "Tertiary",
                        ))
                        .and(color_theme(theme.map_each(|theme| theme.error), "Error")),
                )
                .expand(),
            )
            .and(surface_theme(theme.map_each(|theme| theme.surface))),
    )
    .expand()
}

fn surface_theme(theme: Dynamic<SurfaceTheme>) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let on_color = theme.map_each(|theme| theme.on_color);
    Stack::rows(
        Stack::columns(
            swatch(color.clone(), "Surface", on_color.clone())
                .and(swatch(
                    theme.map_each(|theme| theme.dim_color),
                    "Dim Surface",
                    on_color.clone(),
                ))
                .and(swatch(
                    theme.map_each(|theme| theme.bright_color),
                    "Bright Surface",
                    on_color.clone(),
                )),
        )
        .expand()
        .and(
            Stack::columns(
                swatch(
                    theme.map_each(|theme| theme.lowest_container),
                    "Lowest Container",
                    on_color.clone(),
                )
                .and(swatch(
                    theme.map_each(|theme| theme.low_container),
                    "Low Container",
                    on_color.clone(),
                ))
                .and(swatch(
                    theme.map_each(|theme| theme.container),
                    "Container",
                    on_color.clone(),
                ))
                .and(swatch(
                    theme.map_each(|theme| theme.high_container),
                    "High Container",
                    on_color.clone(),
                ))
                .and(swatch(
                    theme.map_each(|theme| theme.highest_container),
                    "Highest Container",
                    on_color.clone(),
                )),
            )
            .expand(),
        )
        .and(
            Stack::columns(
                swatch(on_color.clone(), "On Surface", color.clone())
                    .and(swatch(
                        theme.map_each(|theme| theme.on_color_variant),
                        "On Color Variant",
                        color.clone(),
                    ))
                    .and(swatch(
                        theme.map_each(|theme| theme.outline),
                        "Outline",
                        color.clone(),
                    ))
                    .and(swatch(
                        theme.map_each(|theme| theme.outline_variant),
                        "Outline Variant",
                        color,
                    )),
            )
            .expand(),
        ),
    )
    .expand()
}

fn color_theme(theme: Dynamic<ColorTheme>, label: &str) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let on_color = theme.map_each(|theme| theme.on_color);
    let container = theme.map_each(|theme| theme.container);
    let on_container = theme.map_each(|theme| theme.on_container);
    Stack::rows(
        swatch(color.clone(), label, on_color.clone())
            .and(swatch(
                on_color.clone(),
                &format!("On {label}"),
                color.clone(),
            ))
            .and(swatch(
                container.clone(),
                &format!("{label} Container"),
                on_container.clone(),
            ))
            .and(swatch(
                on_container,
                &format!("On {label} Container"),
                container,
            )),
    )
    .expand()
}

fn swatch(background: Dynamic<Color>, label: &str, text: Dynamic<Color>) -> impl MakeWidget {
    Label::new(label)
        .with(&TextColor, text)
        .with(&WidgetBackground, background)
        .fit_horizontally()
        .fit_vertically()
        .expand()
}
