use figures::{Figure, Size};

use crate::Scaled;

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

impl<T: Into<Figure<f32, Scaled>> + Clone> Surround<T> {
    /// Returns the minimum width that this surround will occupy.
    #[must_use]
    pub fn minimum_width(&self) -> Figure<f32, Scaled> {
        self.left.clone().map(T::into).unwrap_or_default()
            + self.right.clone().map(T::into).unwrap_or_default()
    }

    /// Returns the minimum height that this surround will occupy.
    #[must_use]
    pub fn minimum_height(&self) -> Figure<f32, Scaled> {
        self.top.clone().map(T::into).unwrap_or_default()
            + self.bottom.clone().map(T::into).unwrap_or_default()
    }

    /// Returns the minimum [`Size`] that this surround will occupy.
    #[must_use]
    pub fn minimum_size(&self) -> Size<f32, Scaled> {
        Size::from_figures(self.minimum_width(), self.minimum_height())
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
