use std::fmt::Debug;

use palette::Srgba;
use stylecs::{FallbackComponent, StyleComponent};

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
    pub light_color: Srgba,
    /// The color used when the current system theme is [`SystemTheme::Dark`].
    pub dark_color: Srgba,
}

impl ColorPair {
    /// Returns a copy of the color pair, replacing each colors alpha channel
    /// with the value provided (0.0-1.0 range).
    #[must_use]
    pub const fn with_alpha(mut self, alpha: f32) -> Self {
        self.light_color.alpha = alpha;
        self.dark_color.alpha = alpha;
        self
    }
}

impl From<Srgba> for ColorPair {
    fn from(color: Srgba) -> Self {
        Self {
            light_color: color,
            dark_color: color,
        }
    }
}

impl ColorPair {
    /// Returns color corresponding to `system_theme`.
    #[must_use]
    pub const fn themed_color(&self, system_theme: &SystemTheme) -> Srgba {
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
            light_color: Srgba::new(0., 0., 0., 1.),
            dark_color: Srgba::new(1., 1., 1., 1.),
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
            light_color: Srgba::new(1., 1., 1., 1.),
            dark_color: Srgba::new(0., 0., 0., 1.),
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
