use std::{any::TypeId, ops::Deref, sync::Arc};

use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    renderer::Renderer,
    stylecs::Points,
    AnyWidget, Gooey, Transmogrifier,
};
use gooey_widgets::{button::ButtonTransmogrifier, container::ContainerTransmogrifier};

mod widgets;

pub struct Rasterizer<R: Renderer> {
    ui: Arc<Gooey<Self>>,
    renderer: Option<R>,
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyWidgetTransmogrifier = Box<dyn AnyWidgetRasterizer<R>>;
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
        let mut frontend = Self {
            ui: Arc::new(ui),
            renderer: None,
        };

        frontend.register_transmogrifier(ButtonTransmogrifier);
        frontend.register_transmogrifier(ContainerTransmogrifier);

        frontend
    }

    pub fn render(&self, scene: R) {
        let size = scene.size();

        if let Some(transmogrifier) = self
            .ui
            .transmogrifiers
            .get(&self.ui.root_widget().widget_type_id())
            .map(|b| b.as_ref())
        {
            transmogrifier.render(
                &Rasterizer {
                    ui: self.ui.clone(),
                    renderer: Some(scene),
                },
                self.ui.root_widget(),
                Rect::new(Point2D::default(), size),
            );
        }
    }

    pub fn register_transmogrifier<M: WidgetRasterizer<R> + Send + Sync + 'static>(
        &mut self,
        transmogrifier: M,
    ) {
        Arc::get_mut(&mut self.ui)
            .expect("couldn't acquire ui as mutable. Do not store any references to Rasterizers.")
            .transmogrifiers
            .insert(TypeId::of::<M::Widget>(), Box::new(transmogrifier));
    }

    pub fn transmogrifier(
        &self,
        widget_type_id: &TypeId,
    ) -> Option<&'_ dyn AnyWidgetRasterizer<R>> {
        self.ui
            .transmogrifiers
            .get(widget_type_id)
            .map(|b| b.as_ref())
    }

    pub fn root_transmogrifier(&'_ self) -> Option<&'_ dyn AnyWidgetRasterizer<R>> {
        self.transmogrifier(&self.ui.root_widget().widget_type_id())
    }
}

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> {
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
