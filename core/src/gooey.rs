use euclid::Size2D;

use crate::{AnyWidget, Points, Widget, WidgetLayout, WidgetState};

pub struct Gooey {
    root: Box<dyn AnyWidget>,
}

impl Gooey {
    pub fn new<W: Widget>(root: W) -> Self {
        Self {
            root: Box::new(WidgetState {
                widget: root,
                state: None,
            }),
        }
    }

    pub fn update(&mut self) -> bool {
        self.root.update()
    }

    pub fn layout_within(&'_ self, size: Size2D<f32, Points>) -> Vec<WidgetLayout<'_>> {
        self.root.layout_within(size)
    }

    pub fn root_widget(&self) -> &dyn AnyWidget {
        self.root.as_ref()
    }
}
