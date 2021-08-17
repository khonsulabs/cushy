use std::fmt::{Debug, Display};

use palette::{FromColor, Hsl, Hsla, Hsv, Hsva, Lab, Laba, Shade, Srgb, Srgba};
use stylecs::{FallbackComponent, StyleComponent};

/// An Srgba color.
#[derive(Debug, Clone, Copy, PartialEq)]
#[must_use]
pub struct Color {
    /// The red channel. Valid values are from 0.0 to 1.0 inclusive.
    pub red: f32,
    /// The green channel. Valid values are from 0.0 to 1.0 inclusive.
    pub green: f32,
    /// The blue channel. Valid values are from 0.0 to 1.0 inclusive.
    pub blue: f32,
    /// The alpha channel. Valid values are from 0.0 to 1.0 inclusive, where 1.0 is fully opaque.
    pub alpha: f32,
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

impl Color {
    /// Creates a new color with SRGBA components `red`, `green`, `blue`, and
    /// `alpha` ranging from 0.0-1.0.
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Creates a new color with SRGBA components `red`, `green`, `blue`, and
    /// `alpha` ranging from 0-255.
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
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    pub fn as_css_string(&self) -> String {
        format!(
            "rgba({}, {}, {}, {:.03})",
            (self.red * 255.).clamp(0., 255.).round() as u8,
            (self.green * 255.).clamp(0., 255.).round() as u8,
            (self.blue * 255.).clamp(0., 255.).round() as u8,
            self.alpha
        )
    }

    /// Returns a gray color with `lightness` in the red, green, and blue channels and an alpha of 1.0.
    pub const fn gray(lightness: f32) -> Self {
        Self::new(lightness, lightness, lightness, 1.0)
    }

    /// Returns a new color with the same red, green, and blue values, but
    /// replacing the current alpha with the parameter.
    pub const fn with_alpha(&self, alpha: f32) -> Self {
        Self::new(self.red, self.green, self.blue, alpha)
    }

    /// Returns this color after lightening it by `factor`. Uses the [`Laba`]
    /// colorspace for luminance calcluations.
    pub fn lighten(self, factor: f32) -> Self {
        let lab = Laba::from(self);
        Self::from(lab.lighten(factor))
    }

    /// Returns this color after darkening it by `factor`. Uses the [`Laba`]
    /// colorspace for luminance calcluations.
    pub fn darken(self, factor: f32) -> Self {
        let lab = Laba::from(self);
        Self::from(lab.darken(factor))
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_css_string())
    }
}

impl From<Srgba> for Color {
    fn from(color: Srgba) -> Self {
        Self {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    }
}

impl From<Srgb> for Color {
    fn from(color: Srgb) -> Self {
        Self::from(Srgba::new(color.red, color.green, color.blue, 1.0))
    }
}

impl From<Hsl> for Color {
    fn from(color: Hsl) -> Self {
        Self::from(Srgba::from_color(color))
    }
}

impl From<Hsla> for Color {
    fn from(color: Hsla) -> Self {
        Self::from(Srgba::from_color(Hsla::new(
            color.hue,
            color.saturation,
            color.lightness,
            1.0,
        )))
    }
}

impl From<Hsv> for Color {
    fn from(color: Hsv) -> Self {
        Self::from(Srgba::from_color(color))
    }
}

impl From<Hsva> for Color {
    fn from(color: Hsva) -> Self {
        Self::from(Srgba::from_color(Hsva::new(
            color.hue,
            color.saturation,
            color.value,
            1.0,
        )))
    }
}

impl From<Color> for Srgba {
    fn from(color: Color) -> Self {
        Self::new(color.red, color.green, color.blue, color.alpha)
    }
}

impl From<Color> for Srgb {
    fn from(color: Color) -> Self {
        Self::new(color.red, color.green, color.blue)
    }
}

impl From<Lab> for Color {
    fn from(color: Lab) -> Self {
        Self::from(Srgba::from_color(color))
    }
}

impl From<Laba> for Color {
    fn from(color: Laba) -> Self {
        Self::from(Srgba::from_color(color))
    }
}

impl From<Color> for Laba {
    fn from(color: Color) -> Self {
        Self::from_color(Srgba::from(color))
    }
}

impl From<Color> for Lab {
    fn from(color: Color) -> Self {
        Self::from_color(Srgb::from(color))
    }
}

/// The theme variant for the system.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
#[must_use]
pub struct ColorPair {
    /// The color used when the current system theme is [`SystemTheme::Light`].
    pub light_color: Color,
    /// The color used when the current system theme is [`SystemTheme::Dark`].
    pub dark_color: Color,
}

impl ColorPair {
    /// Returns a copy of the color pair, replacing each colors alpha channel
    /// with the value provided (0.0-1.0 range).
    pub const fn with_alpha(mut self, alpha: f32) -> Self {
        self.light_color.alpha = alpha;
        self.dark_color.alpha = alpha;
        self
    }

    /// Returns this pair of colors after lightening the colors by `factor`.
    /// Uses the [`Laba`] colorspace for luminance calcluations.
    pub fn lighten(self, factor: f32) -> Self {
        Self {
            light_color: self.light_color.lighten(factor),
            dark_color: self.dark_color.lighten(factor),
        }
    }

    /// Returns this pair of colors after darkening the colors by `factor`. Uses
    /// the [`Laba`] colorspace for luminance calcluations.
    pub fn darken(self, factor: f32) -> Self {
        Self {
            light_color: self.light_color.darken(factor),
            dark_color: self.dark_color.darken(factor),
        }
    }
}

impl From<Srgba> for ColorPair {
    fn from(color: Srgba) -> Self {
        Self::from(Color::from(color))
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
    pub const fn themed_color(&self, system_theme: SystemTheme) -> Color {
        match system_theme {
            SystemTheme::Light => self.light_color,
            SystemTheme::Dark => self.dark_color,
        }
    }
}

/// The foreground color. Used for text and line/border drawing.
#[derive(Debug, Clone)]
pub struct ForegroundColor(pub ColorPair);
impl StyleComponent for ForegroundColor {
    fn should_be_inherited(&self) -> bool {
        false
    }
}

impl Default for ForegroundColor {
    fn default() -> Self {
        Self(ColorPair {
            light_color: Color::BLACK,
            dark_color: Color::WHITE,
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
            light_color: Color::WHITE,
            dark_color: Color::BLACK,
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

impl Color {
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ALICEBLUE: Self = Self::new(240. / 255., 248. / 255., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ANTIQUEWHITE: Self = Self::new(250. / 255., 235. / 255., 215. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AQUA: Self = Self::new(0., 1., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AQUAMARINE: Self = Self::new(127. / 255., 1., 212. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const AZURE: Self = Self::new(240. / 255., 1., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BEIGE: Self = Self::new(245. / 255., 245. / 255., 220. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BISQUE: Self = Self::new(1., 228. / 255., 196. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLACK: Self = Self::new(0., 0., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLANCHEDALMOND: Self = Self::new(1., 235. / 255., 205. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLUE: Self = Self::new(0., 0., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BLUEVIOLET: Self = Self::new(138. / 255., 43. / 255., 226. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BROWN: Self = Self::new(165. / 255., 42. / 255., 42. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const BURLYWOOD: Self = Self::new(222. / 255., 184. / 255., 135. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CADETBLUE: Self = Self::new(95. / 255., 158. / 255., 160. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CHARTREUSE: Self = Self::new(127. / 255., 1., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CHOCOLATE: Self = Self::new(210. / 255., 105. / 255., 30. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CLEAR_BLACK: Self = Self::new(0., 0., 0., 0.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CLEAR_WHITE: Self = Self::new(1., 1., 1., 0.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORAL: Self = Self::new(1., 127. / 255., 80. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORNFLOWERBLUE: Self = Self::new(100. / 255., 149. / 255., 237. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CORNSILK: Self = Self::new(1., 248. / 255., 220. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CRIMSON: Self = Self::new(220. / 255., 20. / 255., 60. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const CYAN: Self = Self::new(0., 1., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKBLUE: Self = Self::new(0., 0., 139. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKCYAN: Self = Self::new(0., 139. / 255., 139. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGOLDENROD: Self = Self::new(184. / 255., 134. / 255., 11. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGRAY: Self = Self::new(169. / 255., 169. / 255., 169. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGREEN: Self = Self::new(0., 100. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKGREY: Self = Self::new(169. / 255., 169. / 255., 169. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKKHAKI: Self = Self::new(189. / 255., 183. / 255., 107. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKMAGENTA: Self = Self::new(139. / 255., 0., 139. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKOLIVEGREEN: Self = Self::new(85. / 255., 107. / 255., 47. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKORANGE: Self = Self::new(1., 140. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKORCHID: Self = Self::new(153. / 255., 50. / 255., 204. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKRED: Self = Self::new(139. / 255., 0., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSALMON: Self = Self::new(233. / 255., 150. / 255., 122. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSEAGREEN: Self = Self::new(143. / 255., 188. / 255., 143. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEBLUE: Self = Self::new(72. / 255., 61. / 255., 139. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEGRAY: Self = Self::new(47. / 255., 79. / 255., 79. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKSLATEGREY: Self = Self::new(47. / 255., 79. / 255., 79. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKTURQUOISE: Self = Self::new(0., 206. / 255., 209. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DARKVIOLET: Self = Self::new(148. / 255., 0., 211. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DEEPPINK: Self = Self::new(1., 20. / 255., 147. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DEEPSKYBLUE: Self = Self::new(0., 191. / 255., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DIMGRAY: Self = Self::new(105. / 255., 105. / 255., 105. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DIMGREY: Self = Self::new(105. / 255., 105. / 255., 105. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const DODGERBLUE: Self = Self::new(30. / 255., 144. / 255., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FIREBRICK: Self = Self::new(178. / 255., 34. / 255., 34. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FLORALWHITE: Self = Self::new(1., 250. / 255., 240. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FORESTGREEN: Self = Self::new(34. / 255., 139. / 255., 34. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const FUCHSIA: Self = Self::new(1., 0., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GAINSBORO: Self = Self::new(220. / 255., 220. / 255., 220. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GHOSTWHITE: Self = Self::new(248. / 255., 248. / 255., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GOLD: Self = Self::new(1., 215. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GOLDENROD: Self = Self::new(218. / 255., 165. / 255., 32. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GRAY: Self = Self::new(128. / 255., 128. / 255., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREEN: Self = Self::new(0., 128. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREENYELLOW: Self = Self::new(173. / 255., 1., 47. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const GREY: Self = Self::new(128. / 255., 128. / 255., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const HONEYDEW: Self = Self::new(240. / 255., 1., 240. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const HOTPINK: Self = Self::new(1., 105. / 255., 180. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const INDIANRED: Self = Self::new(205. / 255., 92. / 255., 92. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const INDIGO: Self = Self::new(75. / 255., 0., 130. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const IVORY: Self = Self::new(1., 1., 240. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const KHAKI: Self = Self::new(240. / 255., 230. / 255., 140. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAVENDER: Self = Self::new(230. / 255., 230. / 255., 250. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAVENDERBLUSH: Self = Self::new(1., 240. / 255., 245. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LAWNGREEN: Self = Self::new(124. / 255., 252. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LEMONCHIFFON: Self = Self::new(1., 250. / 255., 205. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTBLUE: Self = Self::new(173. / 255., 216. / 255., 230. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTCORAL: Self = Self::new(240. / 255., 128. / 255., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTCYAN: Self = Self::new(224. / 255., 1., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGOLDENRODYELLOW: Self = Self::new(250. / 255., 250. / 255., 210. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGRAY: Self = Self::new(211. / 255., 211. / 255., 211. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGREEN: Self = Self::new(144. / 255., 238. / 255., 144. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTGREY: Self = Self::new(211. / 255., 211. / 255., 211. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTPINK: Self = Self::new(1., 182. / 255., 193. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSALMON: Self = Self::new(1., 160. / 255., 122. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSEAGREEN: Self = Self::new(32. / 255., 178. / 255., 170. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSKYBLUE: Self = Self::new(135. / 255., 206. / 255., 250. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSLATEGRAY: Self = Self::new(119. / 255., 136. / 255., 153. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSLATEGREY: Self = Self::new(119. / 255., 136. / 255., 153. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTSTEELBLUE: Self = Self::new(176. / 255., 196. / 255., 222. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIGHTYELLOW: Self = Self::new(1., 1., 224. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIME: Self = Self::new(0., 1., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LIMEGREEN: Self = Self::new(50. / 255., 205. / 255., 50. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const LINEN: Self = Self::new(250. / 255., 240. / 255., 230. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MAGENTA: Self = Self::new(1., 0., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MAROON: Self = Self::new(128. / 255., 0., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMAQUAMARINE: Self = Self::new(102. / 255., 205. / 255., 170. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMBLUE: Self = Self::new(0., 0., 205. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMORCHID: Self = Self::new(186. / 255., 85. / 255., 211. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMPURPLE: Self = Self::new(147. / 255., 112. / 255., 219. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSEAGREEN: Self = Self::new(60. / 255., 179. / 255., 113. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSLATEBLUE: Self = Self::new(123. / 255., 104. / 255., 238. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMSPRINGGREEN: Self = Self::new(0., 250. / 255., 154. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMTURQUOISE: Self = Self::new(72. / 255., 209. / 255., 204. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MEDIUMVIOLETRED: Self = Self::new(199. / 255., 21. / 255., 133. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MIDNIGHTBLUE: Self = Self::new(25. / 255., 25. / 255., 112. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MINTCREAM: Self = Self::new(245. / 255., 1., 250. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MISTYROSE: Self = Self::new(1., 228. / 255., 225. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const MOCCASIN: Self = Self::new(1., 228. / 255., 181. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const NAVAJOWHITE: Self = Self::new(1., 222. / 255., 173. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const NAVY: Self = Self::new(0., 0., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLDLACE: Self = Self::new(253. / 255., 245. / 255., 230. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLIVE: Self = Self::new(128. / 255., 128. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const OLIVEDRAB: Self = Self::new(107. / 255., 142. / 255., 35. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORANGE: Self = Self::new(1., 165. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORANGERED: Self = Self::new(1., 69. / 255., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ORCHID: Self = Self::new(218. / 255., 112. / 255., 214. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEGOLDENROD: Self = Self::new(238. / 255., 232. / 255., 170. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEGREEN: Self = Self::new(152. / 255., 251. / 255., 152. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALETURQUOISE: Self = Self::new(175. / 255., 238. / 255., 238. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PALEVIOLETRED: Self = Self::new(219. / 255., 112. / 255., 147. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PAPAYAWHIP: Self = Self::new(1., 239. / 255., 213. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PEACHPUFF: Self = Self::new(1., 218. / 255., 185. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PERU: Self = Self::new(205. / 255., 133. / 255., 63. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PINK: Self = Self::new(1., 192. / 255., 203. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PLUM: Self = Self::new(221. / 255., 160. / 255., 221. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const POWDERBLUE: Self = Self::new(176. / 255., 224. / 255., 230. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const PURPLE: Self = Self::new(128. / 255., 0., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const REBECCAPURPLE: Self = Self::new(102. / 255., 51. / 255., 153. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const RED: Self = Self::new(1., 0., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ROSYBROWN: Self = Self::new(188. / 255., 143. / 255., 143. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const ROYALBLUE: Self = Self::new(65. / 255., 105. / 255., 225. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SADDLEBROWN: Self = Self::new(139. / 255., 69. / 255., 19. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SALMON: Self = Self::new(250. / 255., 128. / 255., 114. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SANDYBROWN: Self = Self::new(244. / 255., 164. / 255., 96. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SEAGREEN: Self = Self::new(46. / 255., 139. / 255., 87. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SEASHELL: Self = Self::new(1., 245. / 255., 238. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SIENNA: Self = Self::new(160. / 255., 82. / 255., 45. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SILVER: Self = Self::new(192. / 255., 192. / 255., 192. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SKYBLUE: Self = Self::new(135. / 255., 206. / 255., 235. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEBLUE: Self = Self::new(106. / 255., 90. / 255., 205. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEGRAY: Self = Self::new(112. / 255., 128. / 255., 144. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SLATEGREY: Self = Self::new(112. / 255., 128. / 255., 144. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SNOW: Self = Self::new(1., 250. / 255., 250. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const SPRINGGREEN: Self = Self::new(0., 1., 127. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const STEELBLUE: Self = Self::new(70. / 255., 130. / 255., 180. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TAN: Self = Self::new(210. / 255., 180. / 255., 140. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TEAL: Self = Self::new(0., 128. / 255., 128. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const THISTLE: Self = Self::new(216. / 255., 191. / 255., 216. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TOMATO: Self = Self::new(1., 99. / 255., 71. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const TURQUOISE: Self = Self::new(64. / 255., 224. / 255., 208. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const VIOLET: Self = Self::new(238. / 255., 130. / 255., 238. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHEAT: Self = Self::new(245. / 255., 222. / 255., 179. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHITE: Self = Self::new(1., 1., 1., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const WHITESMOKE: Self = Self::new(245. / 255., 245. / 255., 245. / 255., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const YELLOW: Self = Self::new(1., 1., 0., 1.);
    /// Equivalent to the [CSS color keywords](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value) of the same name.
    pub const YELLOWGREEN: Self = Self::new(154. / 255., 205. / 255., 50. / 255., 1.);
}

/// The color to highlight selections.
#[derive(Debug, Clone)]
pub struct SelectionColor(pub ColorPair);
impl StyleComponent for SelectionColor {}

impl From<SelectionColor> for ColorPair {
    fn from(color: SelectionColor) -> Self {
        color.0
    }
}

impl FallbackComponent for SelectionColor {
    type Fallback = ForegroundColor;
    type Value = ColorPair;

    fn value(&self) -> Option<&ColorPair> {
        Some(&self.0)
    }
}
