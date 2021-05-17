use gooey_core::{
    euclid::Length,
    stylecs::{Points, Surround},
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

    pub fn pad_left<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.left = Some(padding.into().get());
        self
    }

    pub fn pad_right<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.right = Some(padding.into().get());
        self
    }

    pub fn pad_top<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.top = Some(padding.into().get());
        self
    }

    pub fn pad_bottom<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.bottom = Some(padding.into().get());
        self
    }
}

impl Widget for Container {
    type TransmogrifierEvent = ();
}

pub struct ContainerTransmogrifier;
