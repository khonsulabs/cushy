use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

use palette::{Hsl, Hsla, Hsv, Hsva, Srgb, Srgba};
use stylecs::{FallbackComponent, StyleComponent};

/// An Srgba color.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Color(pub Srgba);

impl Color {
    /// Creates a new color with SRGBA components `red`, `green`, `blue`, and
    /// `alpha` ranging from 0.0-1.0.
    #[must_use]
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self(Srgba::new(red, green, blue, alpha))
    }

    /// Creates a new color with SRGBA components `red`, `green`, `blue`, and
    /// `alpha` ranging from 0-255.
    #[must_use]
    pub fn new_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self::new(
            f32::from(red) / 255.,
            f32::from(green) / 255.,
            f32::from(blue) / 255.,
            f32::from(alpha) / 255.,
        )
    }

    /// Formats the color for CSS.
    #[must_use]
    pub fn to_css_string(&self) -> String {
        format!(
            "rgba({:.03}, {:.03}, {:.03}, {:.03})",
            self.red, self.green, self.blue, self.alpha
        )
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_css_string())
    }
}

impl From<Srgba> for Color {
    fn from(color: Srgba) -> Self {
        Self(color)
    }
}

impl From<Srgb> for Color {
    fn from(color: Srgb) -> Self {
        Self(Srgba::new(color.red, color.green, color.blue, 1.0))
    }
}

impl From<Hsl> for Color {
    fn from(color: Hsl) -> Self {
        Self(Srgba::from(color))
    }
}

impl From<Hsla> for Color {
    fn from(color: Hsla) -> Self {
        Self(Srgba::from(Hsla::new(
            color.hue,
            color.saturation,
            color.lightness,
            1.0,
        )))
    }
}

impl From<Hsv> for Color {
    fn from(color: Hsv) -> Self {
        Self(Srgba::from(color))
    }
}

impl From<Hsva> for Color {
    fn from(color: Hsva) -> Self {
        Self(Srgba::from(Hsva::new(
            color.hue,
            color.saturation,
            color.value,
            1.0,
        )))
    }
}

impl Deref for Color {
    type Target = Srgba;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Color {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The theme variant for the system.
#[derive(Debug, Clone)]
pub enum SystemTheme {
    /// A light theme.
    Light,
    /// A dark theme.
    Dark,
}

impl StyleComponent for SystemTheme {
    fn should_be_inherited(&self) -> bool {
        true
    }
}

impl Default for SystemTheme {
    fn default() -> Self {
        // So tempted to make this dark.
        Self::Light
    }
}

/// A pair of colors, one for each [`SystemTheme`] variant.
#[derive(Debug, Clone, Default, Copy)]
pub struct ColorPair {
    /// The color used when the current system theme is [`SystemTheme::Light`].
    pub light_color: Color,
    /// The color used when the current system theme is [`SystemTheme::Dark`].
    pub dark_color: Color,
}

impl ColorPair {
    /// Returns a copy of the color pair, replacing each colors alpha channel
    /// with the value provided (0.0-1.0 range).
    #[must_use]
    pub const fn with_alpha(mut self, alpha: f32) -> Self {
        self.light_color.0.alpha = alpha;
        self.dark_color.0.alpha = alpha;
        self
    }
}

impl From<Srgba> for ColorPair {
    fn from(color: Srgba) -> Self {
        Self::from(Color(color))
    }
}

impl From<Color> for ColorPair {
    fn from(color: Color) -> Self {
        Self {
            light_color: color,
            dark_color: color,
        }
    }
}

impl ColorPair {
    /// Returns color corresponding to `system_theme`.
    #[must_use]
    pub const fn themed_color(&self, system_theme: &SystemTheme) -> Color {
        match system_theme {
            SystemTheme::Light => self.light_color,
            SystemTheme::Dark => self.dark_color,
        }
    }
}

/// The foreground color. Used for text and line/border drawing.
#[derive(Debug, Clone)]
pub struct ForegroundColor(pub ColorPair);
impl StyleComponent for ForegroundColor {}

impl Default for ForegroundColor {
    fn default() -> Self {
        Self(ColorPair {
            light_color: Color::new(0., 0., 0., 1.),
            dark_color: Color::new(1., 1., 1., 1.),
        })
    }
}

impl From<ForegroundColor> for ColorPair {
    fn from(color: ForegroundColor) -> Self {
        color.0
    }
}

impl FallbackComponent for ForegroundColor {
    type Fallback = Self;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}

/// The background color. Used for shape fills. Is not inherited.
#[derive(Debug, Clone)]
pub struct BackgroundColor(pub ColorPair);
impl StyleComponent for BackgroundColor {
    fn should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for BackgroundColor {
    fn default() -> Self {
        Self(ColorPair {
            light_color: Color::new(1., 1., 1., 1.),
            dark_color: Color::new(0., 0., 0., 1.),
        })
    }
}

impl From<BackgroundColor> for ColorPair {
    fn from(color: BackgroundColor) -> Self {
        color.0
    }
}

impl FallbackComponent for BackgroundColor {
    type Fallback = Self;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}

/// The foreground color. Used for text and line/border drawing.
#[derive(Debug, Clone)]
pub struct TextColor(pub ColorPair);
impl StyleComponent for TextColor {}

impl From<TextColor> for ColorPair {
    fn from(color: TextColor) -> Self {
        color.0
    }
}

impl FallbackComponent for TextColor {
    type Fallback = ForegroundColor;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}
