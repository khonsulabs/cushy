use euclid::{Length, Scale};
use stylecs::{Pixels, Points, Style, StyleComponent};

/// The width of lines stroked/drawn. Default is `1.` [`Points`].
#[derive(Debug, Copy)]
pub struct LineWidth<Unit>(pub Length<f32, Unit>);

impl<Unit> Clone for LineWidth<Unit> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl Default for LineWidth<Points> {
    fn default() -> Self {
        Self::new(1.)
    }
}

impl<Unit> LineWidth<Unit> {
    /// Creates a new `LineWidth` using `value` in `Unit`.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(Length::new(value))
    }

    /// Returns the line width's raw value.
    #[must_use]
    pub fn get(&self) -> f32 {
        self.0.get()
    }

    /// Returns the line width as a type-safe measurement.
    #[must_use]
    pub const fn length(&self) -> Length<f32, Unit> {
        self.0
    }
}

impl StyleComponent<Points> for LineWidth<Points> {
    fn scale(&self, scale: Scale<f32, Points, Pixels>, map: &mut Style<Pixels>) {
        let value = self.0 * scale;
        map.push(LineWidth(value));
    }
}

impl StyleComponent<Pixels> for LineWidth<Pixels> {
    fn scale(&self, _scale: Scale<f32, Pixels, Pixels>, map: &mut Style<Pixels>) {
        map.push(Self(self.0));
    }
}
