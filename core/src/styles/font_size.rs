use euclid::{Length, Scale};
use stylecs::{Pixels, Points, Style, StyleComponent};

/// The font size for drawing text.
#[derive(Debug, Copy)]
pub struct FontSize<Unit>(pub Length<f32, Unit>);

impl<Unit> Clone for FontSize<Unit> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl Default for FontSize<Points> {
    fn default() -> Self {
        Self::new(14.)
    }
}

impl<Unit> FontSize<Unit> {
    /// Creates a new `FontSize` using `value` in `Unit`.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(Length::new(value))
    }

    /// Returns the raw font size value.
    #[must_use]
    pub fn get(&self) -> f32 {
        self.0.get()
    }

    /// Returns the font size as a type-safe measurement.
    #[must_use]
    pub const fn length(&self) -> Length<f32, Unit> {
        self.0
    }
}

impl StyleComponent<Points> for FontSize<Points> {
    fn scale(&self, scale: Scale<f32, Points, Pixels>, map: &mut Style<Pixels>) {
        let value = self.0 * scale;
        map.push(FontSize(value));
    }
}

impl StyleComponent<Pixels> for FontSize<Pixels> {
    fn scale(&self, _scale: Scale<f32, Pixels, Pixels>, map: &mut Style<Pixels>) {
        map.push(Self(self.0));
    }
}
