use std::fmt::Debug;

use figures::units::Lp;
pub use figures::{Point, Rect, Size};

use crate::style::{Color, Dimension};

pub trait Drawable<Unit>
where
    Unit: crate::math::ScreenUnit,
{
    fn fill_rect(&mut self, rect: Rect<Unit>);
    fn draw_text(
        &mut self,
        text: &str,
        first_baseline_origin: Point<Unit>,
        maximum_width: Option<Unit>,
    );
    fn measure_text(&mut self, text: &str, maximum_width: Option<Unit>) -> TextMetrics<Unit>;
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
