use euclid::{Length, Size2D};

/// Measurements that surrouned a box/rect.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Surround<Unit> {
    /// The left measurement.
    pub left: Option<Length<f32, Unit>>,
    /// The top measurement.
    pub top: Option<Length<f32, Unit>>,
    /// The right measurement.
    pub right: Option<Length<f32, Unit>>,
    /// The bottom measurement.
    pub bottom: Option<Length<f32, Unit>>,
}

impl<Unit> Surround<Unit> {
    /// Returns the minimum width that this surround will occupy.
    #[must_use]
    pub fn minimum_width(&self) -> Length<f32, Unit> {
        self.left.unwrap_or_default() + self.right.unwrap_or_default()
    }

    /// Returns the minimum height that this surround will occupy.
    #[must_use]
    pub fn minimum_height(&self) -> Length<f32, Unit> {
        self.top.unwrap_or_default() + self.bottom.unwrap_or_default()
    }

    /// Returns the minimum [`Size2D`] that this surround will occupy.
    #[must_use]
    pub fn minimum_size(&self) -> Size2D<f32, Unit> {
        Size2D::from_lengths(self.minimum_width(), self.minimum_height())
    }
}
