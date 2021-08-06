use std::ops::{Deref, DerefMut};

use euclid::Length;
use stylecs::StyleComponent;

use super::Surround;
use crate::Points;

/// Adds padding (spacing) around a widget.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Padding(pub Surround<Length<f32, Points>>);

impl StyleComponent for Padding {
    fn should_be_inherited(&self) -> bool {
        false
    }

    fn merge(&self, other: &Self) -> Self {
        Self(Surround {
            left: self.left.or(other.left),
            top: self.top.or(other.top),
            right: self.right.or(other.right),
            bottom: self.bottom.or(other.bottom),
        })
    }
}

impl Padding {
    /// Returns a padding builder.
    pub fn build() -> Builder {
        Builder::default()
    }
}

impl Deref for Padding {
    type Target = Surround<Length<f32, Points>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Padding {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Builds [`Padding`].
#[derive(Default)]
#[must_use]
pub struct Builder {
    padding: Padding,
}

impl Builder {
    /// Sets the left padding to `points`.
    pub fn left(mut self, points: f32) -> Self {
        self.padding.left = Some(Length::new(points));
        self
    }

    /// Sets the right padding to `points`.
    pub fn right(mut self, points: f32) -> Self {
        self.padding.right = Some(Length::new(points));
        self
    }

    /// Sets the top padding to `points`.
    pub fn top(mut self, points: f32) -> Self {
        self.padding.top = Some(Length::new(points));
        self
    }

    /// Sets the bottom padding to `points`.
    pub fn bottom(mut self, points: f32) -> Self {
        self.padding.bottom = Some(Length::new(points));
        self
    }

    /// Returns the built padding.
    pub fn finish(self) -> Padding {
        self.padding
    }
}
