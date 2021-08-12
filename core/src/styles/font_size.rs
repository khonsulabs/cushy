use figures::Figure;
use stylecs::StyleComponent;

use crate::Points;

/// The font size for drawing text.
#[derive(Debug, Copy, Clone)]
pub struct FontSize(pub Figure<f32, Points>);

impl Default for FontSize {
    fn default() -> Self {
        Self::new(14.)
    }
}

impl FontSize {
    /// Creates a new `FontSize` using `value` in `Unit`.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(Figure::new(value))
    }

    /// Returns the raw font size value.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0.get()
    }

    /// Returns the font size as a type-safe measurement.
    #[must_use]
    pub const fn length(self) -> Figure<f32, Points> {
        self.0
    }
}

impl StyleComponent for FontSize {}
