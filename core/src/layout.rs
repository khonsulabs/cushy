use euclid::{Rect, Size2D};

use crate::{stylecs::Points, AnyWidget};

pub trait Layout<'a> {
    fn layout_within(&mut self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'a>>;
}

pub struct WidgetLayout<'a> {
    widget: &'a dyn AnyWidget,
    location: Rect<f32, Points>,
}

impl<'a> Layout<'a> for () {
    fn layout_within(&mut self, _size: Size2D<f32, Points>) -> Vec<WidgetLayout<'a>> {
        Vec::default()
    }
}
