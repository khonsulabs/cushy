use gooey_core::{
    euclid::Length, styles::Surround, Frontend, Points, StyledWidget, Widget, WidgetRef,
    WidgetRegistration, WidgetStorage,
};

#[cfg(feature = "gooey-rasterizer")]
mod rasterizer;

#[cfg(feature = "frontend-browser")]
mod browser;

#[derive(Debug)]
pub struct Container {
    pub child: WidgetRegistration,
    pub padding: Surround<Points>,
}

impl From<WidgetRegistration> for Container {
    fn from(child: WidgetRegistration) -> Self {
        Self {
            child,
            padding: Surround::default(),
        }
    }
}

impl Container {
    pub fn new<W: Widget>(child: StyledWidget<W>, storage: &WidgetStorage) -> StyledWidget<Self> {
        StyledWidget::from(storage.register(child))
    }

    pub fn pad_left<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.left = Some(padding.into());
        self
    }

    pub fn pad_right<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.right = Some(padding.into());
        self
    }

    pub fn pad_top<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.top = Some(padding.into());
        self
    }

    pub fn pad_bottom<F: Into<Length<f32, Points>>>(mut self, padding: F) -> Self {
        self.padding.bottom = Some(padding.into());
        self
    }

    pub fn child<W: Widget, F: Frontend>(&self, frontend: F) -> Option<WidgetRef<W>> {
        WidgetRef::new(&self.child, frontend)
    }
}

impl Widget for Container {
    type Command = ();
    type Event = ();

    const CLASS: &'static str = "gooey-container";
}

#[derive(Debug)]
pub struct ContainerTransmogrifier;
