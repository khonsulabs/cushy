use gooey::styles::components::TextColor;
use gooey::styles::{ColorTheme, FixedTheme, InverseTheme, SurfaceTheme, Theme, ThemePair};
use gooey::widget::MakeWidget;
use gooey::widgets::label::LabelBackground;
use gooey::widgets::{Label, Stack};
use gooey::Run;
use kludgine::Color;

fn main() -> gooey::Result {
    let default_theme = ThemePair::default();
    Stack::columns(
        theme(default_theme.dark, "Dark")
            .and(theme(default_theme.light, "Light"))
            .and(fixed_themes(
                default_theme.primary_fixed,
                default_theme.secondary_fixed,
                default_theme.tertiary_fixed,
            )),
    )
    .expand()
    .run()
}

fn fixed_themes(
    primary: FixedTheme,
    secondary: FixedTheme,
    tertiary: FixedTheme,
) -> impl MakeWidget {
    Stack::rows(
        Label::new("Fixed")
            .and(fixed_theme(primary, "Primary"))
            .and(fixed_theme(secondary, "Secondary"))
            .and(fixed_theme(tertiary, "Tertiary")),
    )
    .expand()
}

fn fixed_theme(theme: FixedTheme, label: &str) -> impl MakeWidget {
    Stack::columns(
        swatch(theme.color, &format!("{label} Fixed"), theme.on_color)
            .and(swatch(
                theme.dim_color,
                &format!("Dim {label}"),
                theme.on_color,
            ))
            .and(swatch(
                theme.on_color,
                &format!("On {label} Fixed"),
                theme.color,
            ))
            .and(swatch(
                theme.on_color_variant,
                &format!("Variant On {label} Fixed"),
                theme.color,
            )),
    )
    .expand()
}

fn theme(theme: Theme, label: &str) -> impl MakeWidget {
    Stack::rows(
        Label::new(label)
            .and(
                Stack::columns(
                    color_theme(theme.primary, "Primary")
                        .and(color_theme(theme.secondary, "Secondary"))
                        .and(color_theme(theme.tertiary, "Tertiary"))
                        .and(color_theme(theme.error, "Error")),
                )
                .expand(),
            )
            .and(surface_and_inverse_themes(theme.surface, theme.inverse)),
    )
    .expand()
}

fn surface_and_inverse_themes(theme: SurfaceTheme, inverse: InverseTheme) -> impl MakeWidget {
    Stack::rows(
        Stack::columns(
            swatch(theme.color, "Surface", theme.on_color)
                .and(swatch(theme.dim_color, "Dim Surface", theme.on_color))
                .and(swatch(theme.bright_color, "Bright Surface", theme.on_color)),
        )
        .expand()
        .and(inverse_theme(inverse))
        .and(
            Stack::columns(
                swatch(theme.lowest_container, "Lowest Container", theme.on_color)
                    .and(swatch(theme.low_container, "Low Container", theme.on_color))
                    .and(swatch(theme.container, "Container", theme.on_color))
                    .and(swatch(
                        theme.high_container,
                        "High Container",
                        theme.on_color,
                    ))
                    .and(swatch(
                        theme.highest_container,
                        "Highest Container",
                        theme.on_color,
                    )),
            )
            .expand(),
        )
        .and(
            Stack::columns(
                swatch(theme.on_color, "On Surface", theme.color)
                    .and(swatch(
                        theme.on_color_variant,
                        "On Color Variant",
                        theme.color,
                    ))
                    .and(swatch(theme.outline, "Outline", theme.color))
                    .and(swatch(
                        theme.outline_variant,
                        "Outline Variant",
                        theme.color,
                    )),
            )
            .expand(),
        ),
    )
    .expand()
}

fn inverse_theme(theme: InverseTheme) -> impl MakeWidget {
    Stack::columns(
        swatch(theme.surface, "Inverse Surface", theme.on_surface)
            .and(swatch(
                theme.on_surface,
                "On Inverse Surface",
                theme.surface,
            ))
            .and(swatch(theme.primary, "Inverse Primary", theme.surface)),
    )
    .expand()
}

fn color_theme(theme: ColorTheme, label: &str) -> impl MakeWidget {
    Stack::rows(
        swatch(theme.color, label, theme.on_color)
            .and(swatch(theme.on_color, &format!("On {label}"), theme.color))
            .and(swatch(
                theme.container,
                &format!("{label} Container"),
                theme.on_container,
            ))
            .and(swatch(
                theme.on_container,
                &format!("On {label} Container"),
                theme.container,
            )),
    )
    .expand()
}

fn swatch(background: Color, label: &str, text: Color) -> impl MakeWidget {
    Label::new(label)
        .fit_horizontally()
        .fit_vertically()
        .with(&TextColor, text)
        .with(&LabelBackground, background)
        .expand()
}
