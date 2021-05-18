use stylecs::{palette::Srgba, ColorPair, UnscaledStyleComponent};

/// The foreground color. Used for text and line/border drawing.
#[derive(Debug, Clone)]
pub struct ForegroundColor(pub ColorPair);
impl UnscaledStyleComponent for ForegroundColor {}

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

/// The background color. Used for shape fills. Is not inherited.
#[derive(Debug, Clone)]
pub struct BackgroundColor(pub ColorPair);
impl UnscaledStyleComponent for BackgroundColor {
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
