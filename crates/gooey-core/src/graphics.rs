//! Types for 2d graphics rendering

use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use figures::units::Lp;
use figures::{Point, Rect, Size};

use crate::style::{Color, Dimension};

/// A surface that can have 2d graphics drawn to it.
pub trait Drawable<Unit>: Deref<Target = Options> + DerefMut
where
    Unit: crate::math::ScreenUnit,
{
    /// Fills `rect` with the current [fill options](FillOptions).
    fn fill_rect(&mut self, rect: Rect<Unit>);

    /// Draws `text` at `first_baseline_origin` using the current
    /// [options](Options).
    fn draw_text(
        &mut self,
        text: &str,
        first_baseline_origin: Point<Unit>,
        maximum_width: Option<Unit>,
    );

    /// Measures `text` using the current [options](Options).
    fn measure_text(&mut self, text: &str, maximum_width: Option<Unit>) -> TextMetrics<Unit>;
}

/// Options for a [`Drawable`]s graphics operations.
#[derive(Debug)]
pub struct Options {
    /// Options for fill operations.
    pub fill: FillOptions,
    /// The size that text is rendered at.
    pub font_size: Dimension,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fill: FillOptions::default(),
            font_size: Lp(12).into(),
        }
    }
}

/// Options for a filling graphics operations on a [`Drawable`].
#[derive(Debug)]
pub struct FillOptions {
    /// The color to fill with.
    pub color: Color,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            color: Color::rgba(0, 0, 0, 255),
        }
    }
}

/// Dimensions of a measured block of text.
#[derive(Debug)]
pub struct TextMetrics<Unit> {
    /// The distance above the baseline the text extends.
    pub ascent: Unit,
    /// The distance below the baseline the text extends. This measurement is
    /// typically negative.
    pub descent: Unit,
    /// The full size of the measured text.
    pub size: Size<Unit>,
}
