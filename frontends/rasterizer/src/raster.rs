use std::collections::HashMap;

use gooey_core::{euclid::Rect, styles::Points, WidgetId};

#[derive(Default, Debug)]

pub struct Raster {
    pub order: Vec<WidgetId>,
    pub bounds: HashMap<u32, Rect<f32, Points>>,
}

impl Raster {
    pub fn reset(&mut self) {
        self.order.clear();
        self.bounds.clear();
    }

    pub fn widget_rendered(&mut self, widget: WidgetId, bounds: Rect<f32, Points>) {
        self.bounds.insert(widget.id, bounds);
        self.order.push(widget);
    }
}
