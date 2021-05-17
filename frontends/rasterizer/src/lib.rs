use std::{any::TypeId, ops::Deref, sync::Arc};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    stylecs::Points,
    AnyTransmogrifier, AnyWidget, Gooey, Transmogrifier,
};

pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    renderer: Option<R>,
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyWidgetTransmogrifier = RegisteredTransmogrifier<R>;
    type Context = Self;
}

impl<R: Renderer> Deref for Rasterizer<R> {
    type Target = Option<R>;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}

impl<R: Renderer> Rasterizer<R> {
    pub fn new(ui: Gooey<Self>) -> Self {
        Self {
            ui: Arc::new(ui),
            renderer: None,
        }
    }

    pub fn render(&self, scene: R) {
        let size = scene.size();

        if let Some(transmogrifier) = self.ui.root_transmogrifier() {
            transmogrifier.render(
                &Rasterizer {
                    ui: self.ui.clone(),
                    renderer: Some(scene),
                },
                self.ui.root_widget(),
                Rect::new(Point2D::default(), size),
            );
        } else {
            todo!("Return an error -- unknown widget type")
        }
    }
}

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render(
        &self,
        rasterizer: &Rasterizer<R>,
        widget: &<Self as Transmogrifier<Rasterizer<R>>>::Widget,
        bounds: Rect<f32, Points>,
    );

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn content_size(
        &self,
        widget: &Self::Widget,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

pub trait AnyWidgetRasterizer<R: Renderer>: Send + Sync {
    fn widget_type_id(&self) -> TypeId;
    fn render(&self, rasterizer: &Rasterizer<R>, widget: &dyn AnyWidget, bounds: Rect<f32, Points>);
    fn content_size(
        &self,
        widget: &dyn AnyWidget,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + Send + Sync + 'static,
    R: Renderer,
{
    fn widget_type_id(&self) -> TypeId {
        <T as WidgetRasterizer<R>>::widget_type_id(self)
    }

    fn render(
        &self,
        rasterizer: &Rasterizer<R>,
        widget: &dyn AnyWidget,
        bounds: Rect<f32, Points>,
    ) {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .unwrap();
        <T as WidgetRasterizer<R>>::render(&self, rasterizer, widget, bounds)
    }

    fn content_size(
        &self,
        widget: &dyn AnyWidget,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .unwrap();
        <T as WidgetRasterizer<R>>::content_size(&self, widget, rasterizer, constraints)
    }
}

pub struct RegisteredTransmogrifier<R: Renderer>(pub Box<dyn AnyWidgetRasterizer<R>>);

impl<R: Renderer> Deref for RegisteredTransmogrifier<R> {
    type Target = Box<dyn AnyWidgetRasterizer<R>>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
    }
}

impl<R: Renderer> AnyTransmogrifier for RegisteredTransmogrifier<R> {
    fn widget_type_id(&self) -> TypeId {
        AnyWidgetRasterizer::widget_type_id(self.0.as_ref())
    }
}

#[macro_export]
macro_rules! make_rasterized {
    ($transmogrifier:ident) => {
        impl<R: $crate::Renderer> From<$transmogrifier> for $crate::RegisteredTransmogrifier<R> {
            fn from(transmogrifier: $transmogrifier) -> Self {
                Self(std::boxed::Box::new(transmogrifier))
            }
        }
    };
}
