use std::marker::PhantomData;

use euclid::{Length, Size2D};

/// Measurements that surrouned a box/rect.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Surround<Unit> {
    /// The left measurement.
    pub left: Option<f32>,
    /// The top measurement.
    pub top: Option<f32>,
    /// The right measurement.
    pub right: Option<f32>,
    /// The bottom measurement.
    pub bottom: Option<f32>,
    _phantom: PhantomData<Unit>,
}

impl<Unit> Surround<Unit> {
    /// Returns the minimum width that this surround will occupy.
    #[must_use]
    pub fn minimum_width(&self) -> Length<f32, Unit> {
        self.left().unwrap_or_default() + self.right().unwrap_or_default()
    }

    /// Returns the minimum height that this surround will occupy.
    #[must_use]
    pub fn minimum_height(&self) -> Length<f32, Unit> {
        self.top().unwrap_or_default() + self.bottom().unwrap_or_default()
    }

    /// Returns the minimum [`Size2D`] that this surround will occupy.
    #[must_use]
    pub fn minimum_size(&self) -> Size2D<f32, Unit> {
        Size2D::from_lengths(self.minimum_width(), self.minimum_height())
    }

    /// Returns the left measurement using the unit-aware type: [`Length`].
    pub fn left(&self) -> Option<Length<f32, Unit>> {
        self.left.map(Length::new)
    }

    /// Returns the right measurement using the unit-aware type: [`Length`].
    pub fn right(&self) -> Option<Length<f32, Unit>> {
        self.right.map(Length::new)
    }

    /// Returns the bottom measurement using the unit-aware type: [`Length`].
    pub fn bottom(&self) -> Option<Length<f32, Unit>> {
        self.bottom.map(Length::new)
    }

    /// Returns the top measurement using the unit-aware type: [`Length`].
    pub fn top(&self) -> Option<Length<f32, Unit>> {
        self.top.map(Length::new)
    }
}
