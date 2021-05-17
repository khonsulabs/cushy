use std::{any::TypeId, marker::PhantomData};

use gooey_core::{
    euclid::{Point2D, Rect},
    renderer::Renderer,
    stylecs::Points,
    AnyWidget, Gooey, Transmogrifier,
};
use gooey_widgets::button::ButtonTransmogrifier;

mod widgets;

pub struct Rasterizer<R: Renderer> {
    ui: Gooey<Self>,
    _phantom: PhantomData<R>,
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyWidgetTransmogrifier = Box<dyn AnyWidgetRasterizer<R>>;
}

impl<R: Renderer> Rasterizer<R> {
    pub fn new(ui: Gooey<Self>) -> Self {
        let mut frontend = Self {
            ui,
            _phantom: PhantomData::default(),
        };

        frontend.register_transmogrifier(ButtonTransmogrifier);

        frontend
    }

    pub fn render(&self, scene: &R) {
        let size = scene.size();

        if let Some(transmogrifier) = self
            .ui
            .transmogrifiers
            .get(&self.ui.root_widget().widget_type_id())
            .map(|b| b.as_ref())
        {
            transmogrifier.render(
                scene,
                self.ui.root_widget(),
                Rect::new(Point2D::default(), size),
            );
        }
    }

    pub fn register_transmogrifier<M: WidgetRasterizer<R> + Send + Sync + 'static>(
        &mut self,
        transmogrifier: M,
    ) {
        self.ui
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
        scene: &R,
        widget: &<Self as Transmogrifier<Rasterizer<R>>>::Widget,
        bounds: Rect<f32, Points>,
    );
}

pub trait AnyWidgetRasterizer<R: Renderer>: Send + Sync {
    fn render(&self, scene: &R, widget: &dyn AnyWidget, bounds: Rect<f32, Points>);
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + Send + Sync + 'static,
    R: Renderer,
{
    fn render(&self, scene: &R, widget: &dyn AnyWidget, bounds: Rect<f32, Points>) {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .unwrap();
        <T as WidgetRasterizer<R>>::render(&self, scene, widget, bounds)
    }
}
