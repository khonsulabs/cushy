use gooey_core::{
    stylecs::{Dimension, Points, Surround},
    AnyWidget, Widget,
};

pub struct Container {
    pub child: Box<dyn AnyWidget>,
    pub padding: Surround<Points>,
}

impl Container {
    pub fn new<W: Widget + AnyWidget>(child: W) -> Self {
        Self {
            child: Box::new(child),
            padding: Surround::default(),
        }
    }

    pub fn pad_left<F: Into<Dimension<Points>>>(mut self, padding: F) -> Self {
        self.padding.left = padding.into();
        self
    }

    pub fn pad_right<F: Into<Dimension<Points>>>(mut self, padding: F) -> Self {
        self.padding.right = padding.into();
        self
    }

    pub fn pad_top<F: Into<Dimension<Points>>>(mut self, padding: F) -> Self {
        self.padding.top = padding.into();
        self
    }

    pub fn pad_bottom<F: Into<Dimension<Points>>>(mut self, padding: F) -> Self {
        self.padding.bottom = padding.into();
        self
    }
}

impl Widget for Container {
    type TransmogrifierEvent = ();
}

pub struct ContainerTransmogrifier;
