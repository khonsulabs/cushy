use std::fmt::Write;

use cushy::figures::units::Lp;
use cushy::kludgine::Color;
use cushy::styles::components::{TextColor, TextSize, WidgetBackground};
use cushy::styles::{
    ColorScheme, ColorSchemeBuilder, ColorSource, ColorTheme, Dimension, FixedTheme, OklabHue,
    SurfaceTheme, Theme, ThemePair,
};
use cushy::value::{Destination, Dynamic, MapEachCloned, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::checkbox::Checkable;
use cushy::widgets::color::ColorSourcePicker;
use cushy::widgets::input::InputValue;
use cushy::widgets::slider::Slidable;
use cushy::widgets::Space;
use cushy::window::ThemeMode;
use cushy::{Cushy, Open, PendingApp};

fn main() -> cushy::Result {
    let app = PendingApp::default();
    theme_editor(app.cushy().clone()).into_window().run_in(app)
}

fn theme_editor(cushy: Cushy) -> impl MakeWidget {
    let (theme_mode, theme_switcher) = dark_mode_picker();

    let scheme = Scheme::from(ColorScheme::default());
    let sources = scheme.map(Dynamic::new);
    let editors = sources.map_labeled(
        |primary| {
            swatch_label("Primary", &primary)
                .and(color_editor(&primary))
                .into_rows()
                .make_widget()
        },
        |label, source| {
            let (enabled, editor) = optional_editor(label, &source);
            let opt_color =
                (&enabled, &source).map_each_cloned(|(enabled, source)| enabled.then_some(source));
            (opt_color, editor)
        },
    );
    let color_scheme_builder = (
        &sources.primary,
        &editors.secondary.0,
        &editors.tertiary.0,
        &editors.error.0,
        &editors.neutral.0,
        &editors.neutral_variant.0,
    )
        .map_each_cloned(
            move |(primary, secondary, tertiary, error, neutral, neutral_variant)| {
                let mut scheme = ColorSchemeBuilder::new(primary);
                scheme.secondary = secondary;
                scheme.tertiary = tertiary;
                scheme.error = error;
                scheme.neutral = neutral;
                scheme.neutral_variant = neutral_variant;
                scheme
            },
        );
    let color_scheme = color_scheme_builder.map_each_cloned(|builder| builder.build());
    color_scheme
        .for_each_cloned(move |scheme| {
            sources.primary.set(scheme.primary);
            sources.secondary.set(scheme.secondary);
            sources.tertiary.set(scheme.tertiary);
            sources.error.set(scheme.error);
            sources.neutral.set(scheme.neutral);
            sources.neutral_variant.set(scheme.neutral_variant);
        })
        .persist();
    let theme = color_scheme.map_each_cloned(ThemePair::from);

    let editors = theme_switcher
        .and(editors.primary)
        .and(editors.secondary.1)
        .and(editors.tertiary.1)
        .and(editors.error.1)
        .and(editors.neutral.1)
        .and(editors.neutral_variant.1)
        .and("Copy to Clipboard".into_button().on_click({
            move |_| {
                if let Some(mut clipboard) = cushy.clipboard_guard() {
                    let builder = color_scheme_builder.get();
                    let mut source = String::default();
                    builder.format_rust_into(&mut source);

                    if let Err(err) = clipboard.set_text(&source) {
                        tracing::error!("Error setting clipboard text: {err}");
                        println!("{source}");
                    }
                }
            }
        }))
        .into_rows()
        .pad()
        .vertical_scroll();

    editors
        .and(fixed_themes(
            theme.map_each(|theme| theme.primary_fixed),
            theme.map_each(|theme| theme.secondary_fixed),
            theme.map_each(|theme| theme.tertiary_fixed),
        ))
        .and(theme_preview(
            theme.map_each(|theme| theme.dark),
            ThemeMode::Dark,
        ))
        .and(theme_preview(
            theme.map_each(|theme| theme.light),
            ThemeMode::Light,
        ))
        .into_columns()
        .themed(theme)
        .pad()
        .expand()
        .themed_mode(theme_mode)
}

struct Scheme<Primary, Other = Primary> {
    primary: Primary,
    secondary: Other,
    tertiary: Other,
    error: Other,
    neutral: Other,
    neutral_variant: Other,
}

impl From<ColorScheme> for Scheme<ColorSource> {
    fn from(scheme: ColorScheme) -> Self {
        Self {
            primary: scheme.primary,
            secondary: scheme.secondary,
            tertiary: scheme.tertiary,
            error: scheme.error,
            neutral: scheme.neutral,
            neutral_variant: scheme.neutral_variant,
        }
    }
}

impl<T> Scheme<T> {
    pub fn map<R>(&self, mut map: impl FnMut(T) -> R) -> Scheme<R>
    where
        T: Clone,
    {
        Scheme {
            primary: map(self.primary.clone()),
            secondary: map(self.secondary.clone()),
            tertiary: map(self.tertiary.clone()),
            error: map(self.error.clone()),
            neutral: map(self.neutral.clone()),
            neutral_variant: map(self.neutral_variant.clone()),
        }
    }
}

impl<Primary, Other> Scheme<Primary, Other> {
    pub fn map_labeled<NewPrimary, NewOther>(
        &self,
        primary: impl FnOnce(Primary) -> NewPrimary,
        mut map: impl FnMut(&str, Other) -> NewOther,
    ) -> Scheme<NewPrimary, NewOther>
    where
        Primary: Clone,
        Other: Clone,
    {
        Scheme {
            primary: primary(self.primary.clone()),
            secondary: map("Secondary", self.secondary.clone()),
            tertiary: map("Tertiary", self.tertiary.clone()),
            error: map("Error", self.error.clone()),
            neutral: map("Netural", self.neutral.clone()),
            neutral_variant: map("Neutral Variant", self.neutral_variant.clone()),
        }
    }
}

fn dark_mode_picker() -> (Dynamic<ThemeMode>, impl MakeWidget) {
    let dark = Dynamic::new(true);
    let theme_mode = dark.map_each(|dark| {
        if *dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        }
    });

    (theme_mode.clone(), dark.into_checkbox("Dark Mode"))
}

fn swatch_label(label: &str, color: &Dynamic<ColorSource>) -> impl MakeWidget {
    Space::colored(color.map_each(|source| source.color(0.5)))
        .width(Lp::mm(1))
        .and(label)
        .into_columns()
}

fn optional_editor(label: &str, color: &Dynamic<ColorSource>) -> (Dynamic<bool>, impl MakeWidget) {
    let enabled = Dynamic::new(false);
    let hide_editor = enabled.map_each(|enabled| !enabled);

    (
        enabled.clone(),
        enabled
            .to_checkbox(swatch_label(label, color))
            .and(color_editor(color).collapse_vertically(hide_editor))
            .into_rows(),
    )
}

fn color_editor(color: &Dynamic<ColorSource>) -> impl MakeWidget {
    let hue = color.map_each_cloned(|color| color.hue.into_positive_degrees());
    hue.for_each_cloned({
        let color = color.clone();
        move |hue| {
            let mut source = color.get();
            source.hue = OklabHue::new(hue);
            color.set(source);
        }
    })
    .persist();

    let hue_text = hue.linked_string();
    let saturation = color.map_each_cloned(|color| color.saturation);
    saturation
        .for_each_cloned({
            let color = color.clone();
            move |saturation| {
                let mut source = color.get();
                source.saturation = saturation;
                color.set(source);
            }
        })
        .persist();
    let saturation_text = saturation.linked_string();

    ColorSourcePicker::new(color.clone())
        .height(Lp::points(100))
        .fit_horizontally()
        .and(hue.slider_between(0., 360.))
        .and(hue_text.into_input())
        .and(saturation.slider())
        .and(saturation_text.into_input())
        .into_rows()
}

fn fixed_themes(
    primary: Dynamic<FixedTheme>,
    secondary: Dynamic<FixedTheme>,
    tertiary: Dynamic<FixedTheme>,
) -> impl MakeWidget {
    "Fixed"
        .and(fixed_theme(primary, "Primary"))
        .and(fixed_theme(secondary, "Secondary"))
        .and(fixed_theme(tertiary, "Tertiary"))
        .into_rows()
        .contain()
        .expand()
}

fn fixed_theme(theme: Dynamic<FixedTheme>, label: &str) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let on_color = theme.map_each(|theme| theme.on_color);

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
        ))
        .into_columns()
        .expand()
        .contain()
        .expand()
}

fn theme_preview(theme: Dynamic<Theme>, mode: ThemeMode) -> impl MakeWidget {
    match mode {
        ThemeMode::Light => "Light",
        ThemeMode::Dark => "Dark",
    }
    .and(
        color_theme(theme.map_each(|theme| theme.primary), "Primary")
            .and(color_theme(
                theme.map_each(|theme| theme.secondary),
                "Secondary",
            ))
            .and(color_theme(
                theme.map_each(|theme| theme.tertiary),
                "Tertiary",
            ))
            .and(color_theme(theme.map_each(|theme| theme.error), "Error"))
            .into_columns()
            .contain()
            .expand(),
    )
    .and(surface_theme(theme.map_each(|theme| theme.surface)))
    .into_rows()
    .contain()
    .themed_mode(mode)
    .expand()
}

fn surface_theme(theme: Dynamic<SurfaceTheme>) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let on_color = theme.map_each(|theme| theme.on_color);

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
        ))
        .into_columns()
        .contain()
        .expand()
        .and(
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
            ))
            .into_columns()
            .contain()
            .expand(),
        )
        .and(
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
                ))
                .into_columns()
                .contain()
                .expand(),
        )
        .into_rows()
        .contain()
        .expand()
}

fn color_theme(theme: Dynamic<ColorTheme>, label: &str) -> impl MakeWidget {
    let color = theme.map_each(|theme| theme.color);
    let dim_color = theme.map_each(|theme| theme.color_dim);
    let bright_color = theme.map_each(|theme| theme.color_bright);
    let on_color = theme.map_each(|theme| theme.on_color);
    let container = theme.map_each(|theme| theme.container);
    let on_container = theme.map_each(|theme| theme.on_container);

    swatch(color.clone(), label, on_color.clone())
        .and(swatch(
            dim_color.clone(),
            &format!("{label} Dim"),
            on_color.clone(),
        ))
        .and(swatch(
            bright_color.clone(),
            &format!("{label} bright"),
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
        ))
        .into_rows()
        .contain()
        .expand()
}

fn swatch(background: Dynamic<Color>, label: &str, text: Dynamic<Color>) -> impl MakeWidget {
    label
        .with(&TextColor, text)
        .with(&TextSize, Dimension::Lp(Lp::points(8)))
        .with(&WidgetBackground, background)
        .fit_horizontally()
        .fit_vertically()
        .expand()
}

trait FormatRust {
    fn format_rust_into(&self, out: &mut String);
}

impl FormatRust for ColorSource {
    fn format_rust_into(&self, out: &mut String) {
        write!(
            out,
            "ColorSource::new({:.1}, {:.1})",
            self.hue.into_degrees(),
            self.saturation
        )
        .expect("writing to string")
    }
}

impl FormatRust for ColorSchemeBuilder {
    fn format_rust_into(&self, source: &mut String) {
        if self.secondary.is_none()
            && self.tertiary.is_none()
            && self.error.is_none()
            && self.neutral.is_none()
            && self.neutral_variant.is_none()
        {
            source.push_str("ColorScheme::from_primary(");
            self.primary.format_rust_into(source);
            source.push(')');
        } else {
            source.push_str("ColorSchemeBuilder::new(");
            self.primary.format_rust_into(source);
            source.push_str(").");
            for (label, color) in [
                self.secondary.map(|secondary| ("secondary", secondary)),
                self.tertiary.map(|color| ("tertiary", color)),
                self.error.map(|color| ("error", color)),
                self.neutral.map(|color| ("neutral", color)),
                self.neutral_variant.map(|color| ("neutral_variant", color)),
            ]
            .into_iter()
            .flatten()
            {
                source.push_str(label);
                source.push('(');
                color.format_rust_into(source);
                source.push_str(").");
            }
            source.push_str("build()");
        }
    }
}

#[test]
fn runs() {
    let theme_editor = || theme_editor(Cushy::default());
    cushy::example!(theme_editor, 1600, 900).untested_still_frame();
}
