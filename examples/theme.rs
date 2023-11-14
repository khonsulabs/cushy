use std::str::FromStr;

use gooey::animation::ZeroToOne;
use gooey::styles::components::{TextColor, WidgetBackground};
use gooey::styles::{
    ColorScheme, ColorSource, ColorTheme, FixedTheme, SurfaceTheme, Theme, ThemePair,
};
use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::{Input, Label, ModeSwitch, Scroll, Slider, Stack, Themed};
use gooey::window::ThemeMode;
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    let scheme = ColorScheme::default();
    let (primary, primary_editor) = color_editor(scheme.primary, "Primary");
    let (secondary, secondary_editor) = color_editor(scheme.secondary, "Secondary");
    let (tertiary, tertiary_editor) = color_editor(scheme.tertiary, "Tertiary");
    let (error, error_editor) = color_editor(scheme.error, "Error");
    let (neutral, neutral_editor) = color_editor(scheme.neutral, "Neutral");
    let (neutral_variant, neutral_variant_editor) =
        color_editor(scheme.neutral_variant, "Neutral Variant");
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
                ThemePair::from(ColorScheme {
                    primary: *primary,
                    secondary: *secondary,
                    tertiary: *tertiary,
                    error: *error,
                    neutral: *neutral,
                    neutral_variant: *neutral_variant,
                })
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
            .and(fixed_themes(
                default_theme.map_each(|theme| theme.primary_fixed),
                default_theme.map_each(|theme| theme.secondary_fixed),
                default_theme.map_each(|theme| theme.tertiary_fixed),
            ))
            .and(theme(
                default_theme.map_each(|theme| theme.dark),
                ThemeMode::Dark,
            ))
            .and(theme(
                default_theme.map_each(|theme| theme.light),
                ThemeMode::Light,
            )),
        ),
    )
    .pad()
    .expand()
    .into_window()
    .with_theme_mode(theme_mode)
    .run()
}

fn dark_mode_slider() -> (Dynamic<ThemeMode>, impl MakeWidget) {
    let theme_mode = Dynamic::default();

    (
        theme_mode.clone(),
        Stack::rows(Label::new("Theme Mode").and(Slider::<ThemeMode>::from_value(theme_mode))),
    )
}

fn create_paired_string<T>(initial_value: T) -> (Dynamic<T>, Dynamic<String>)
where
    T: ToString + PartialEq + FromStr + Default + Send + Sync + 'static,
{
    let float = Dynamic::new(initial_value);
    let text = float.map_each_unique(|f| f.to_string());
    text.for_each(float.with_clone(|float| {
        move |text: &String| {
            let _result = float.try_update(text.parse().unwrap_or_default());
        }
    }));
    (float, text)
}

fn color_editor(
    initial_color: ColorSource,
    label: &str,
) -> (Dynamic<ColorSource>, impl MakeWidget) {
    let (hue, hue_text) = create_paired_string(initial_color.hue.into_degrees());
    let (saturation, saturation_text) = create_paired_string(initial_color.saturation);

    let color =
        (&hue, &saturation).map_each(|(hue, saturation)| ColorSource::new(*hue, *saturation));

    (
        color,
        Stack::rows(
            Label::new(label)
                .and(Slider::<f32>::new(hue, 0., 360.))
                .and(Input::new(hue_text))
                .and(Slider::<ZeroToOne>::from_value(saturation))
                .and(Input::new(saturation_text)),
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
    .contain()
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
    .contain()
    .expand()
}

fn theme(theme: Dynamic<Theme>, mode: ThemeMode) -> impl MakeWidget {
    ModeSwitch::new(
        mode,
        Stack::rows(
            Label::new(match mode {
                ThemeMode::Light => "Light",
                ThemeMode::Dark => "Dark",
            })
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
                .contain()
                .expand(),
            )
            .and(surface_theme(theme.map_each(|theme| theme.surface))),
        )
        .contain(),
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
                    theme.map_each(|theme| theme.bright_color),
                    "Bright Surface",
                    on_color.clone(),
                ))
                .and(swatch(
                    theme.map_each(|theme| theme.dim_color),
                    "Dim Surface",
                    on_color.clone(),
                )),
        )
        .contain()
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
            .contain()
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
                    ))
                    .and(swatch(
                        theme.map_each(|theme| theme.opaque_widget),
                        "Opaque Widget",
                        on_color,
                    )),
            )
            .contain()
            .expand(),
        ),
    )
    .contain()
    .expand()
}

fn color_theme(theme: Dynamic<ColorTheme>, label: &str) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let dim_color = theme.map_each(|theme| theme.color_dim);
    let on_color = theme.map_each(|theme| theme.on_color);
    let container = theme.map_each(|theme| theme.container);
    let on_container = theme.map_each(|theme| theme.on_container);
    Stack::rows(
        swatch(color.clone(), label, on_color.clone())
            .and(swatch(
                dim_color.clone(),
                &format!("{label} Dim"),
                on_color.clone(),
            ))
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
    .contain()
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
