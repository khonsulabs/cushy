use euclid::{Length, Size2D};

use crate::Points;

/// Measurements that surrouned a box/rect.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Surround<T> {
    /// The left measurement.
    pub left: Option<T>,
    /// The top measurement.
    pub top: Option<T>,
    /// The right measurement.
    pub right: Option<T>,
    /// The bottom measurement.
    pub bottom: Option<T>,
}

impl<T: Into<Length<f32, Points>> + Clone> Surround<T> {
    /// Returns the minimum width that this surround will occupy.
    #[must_use]
    pub fn minimum_width(&self) -> Length<f32, Points> {
        self.left.clone().map(T::into).unwrap_or_default()
            + self.right.clone().map(T::into).unwrap_or_default()
    }

    /// Returns the minimum height that this surround will occupy.
    #[must_use]
    pub fn minimum_height(&self) -> Length<f32, Points> {
        self.top.clone().map(T::into).unwrap_or_default()
            + self.bottom.clone().map(T::into).unwrap_or_default()
    }

    /// Returns the minimum [`Size2D`] that this surround will occupy.
    #[must_use]
    pub fn minimum_size(&self) -> Size2D<f32, Points> {
        Size2D::from_lengths(self.minimum_width(), self.minimum_height())
    }
}

impl<T: Clone> From<Option<T>> for Surround<T> {
    fn from(value: Option<T>) -> Self {
        Self {
            left: value.clone(),
            top: value.clone(),
            right: value.clone(),
            bottom: value,
        }
    }
}
