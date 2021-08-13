use std::ops::{Deref, DerefMut};

use figures::Figure;
use stylecs::StyleComponent;

use super::{Color, Surround};
use crate::Scaled;

/// A border around a widget.
#[derive(Default, Debug, Clone)]
#[must_use]
pub struct Border(pub Surround<BorderOptions>);

impl Border {
    /// Returns a new border builder.
    pub fn build() -> Builder {
        Builder::default()
    }

    /// Returns a border with all sides having `options`.
    pub fn uniform(options: BorderOptions) -> Self {
        Self(Surround::from(Some(options)))
    }
}

/// Options for a single side of a [`Border`].
#[derive(Debug, Default, Copy, Clone)]
#[must_use]
pub struct BorderOptions {
    /// The width of the border.
    pub width: Figure<f32, Scaled>,
    /// The color of the border.
    pub color: Color,
}

impl BorderOptions {
    /// Returns a new border with `width` and `color`.
    pub const fn new(width: f32, color: Color) -> Self {
        Self {
            width: Figure::new(width),
            color,
        }
    }
}

impl From<Figure<f32, Scaled>> for BorderOptions {
    fn from(width: Figure<f32, Scaled>) -> Self {
        Self {
            width,
            color: Color::default(),
        }
    }
}

impl From<BorderOptions> for Figure<f32, Scaled> {
    fn from(opts: BorderOptions) -> Self {
        opts.width
    }
}

impl StyleComponent for Border {
    fn should_be_inherited(&self) -> bool {
        false
    }
}

impl Deref for Border {
    type Target = Surround<BorderOptions>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Border {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Builds a [`Border`].
#[derive(Default, Debug)]
#[must_use]
pub struct Builder {
    border: Border,
}

impl Builder {
    /// Sets the left border `options`.
    pub fn left(mut self, options: BorderOptions) -> Self {
        self.border.left = Some(options);
        self
    }

    /// Sets the right border `options`.
    pub fn right(mut self, options: BorderOptions) -> Self {
        self.border.right = Some(options);
        self
    }

    /// Sets the top border `options`.
    pub fn top(mut self, options: BorderOptions) -> Self {
        self.border.top = Some(options);
        self
    }

    /// Sets the bottom border `options`.
    pub fn bottom(mut self, options: BorderOptions) -> Self {
        self.border.bottom = Some(options);
        self
    }

    /// Returns the built border.
    pub fn finish(self) -> Border {
        self.border
    }
}
