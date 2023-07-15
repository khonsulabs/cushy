use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use figures::units::{Lp, UPx};
pub use figures::{Point, Rect, Size};

use crate::style::{Color, Dimension};

pub trait Renderer: Deref<Target = Options> + DerefMut<Target = Options> + Debug {
    type Clipped<'clip>: Renderer
    where
        Self: 'clip;

    fn fill_rect<Unit>(&mut self, rect: Rect<Unit>)
    where
        Unit: crate::math::ScreenUnit;
    fn draw_text<Unit>(
        &mut self,
        text: &str,
        first_baseline_origin: Point<Unit>,
        maximum_width: Option<Unit>,
    ) where
        Unit: crate::math::ScreenUnit;
    fn measure_text<Unit>(&mut self, text: &str, maximum_width: Option<Unit>) -> TextMetrics<Unit>
    where
        Unit: crate::math::ScreenUnit;

    fn clip_to(&mut self, clip: Rect<UPx>) -> Self::Clipped<'_>;

    fn size(&self) -> Size<UPx>;
}

#[derive(Debug)]
pub struct Options {
    pub fill: FillOptions,
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

#[derive(Debug)]
pub struct FillOptions {
    pub color: Color,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self {
            color: Color::rgba(0, 0, 0, 255),
        }
    }
}

#[derive(Debug)]
pub struct TextMetrics<Unit> {
    pub ascent: Unit,
    pub descent: Unit,
    pub size: Size<Unit>,
}
