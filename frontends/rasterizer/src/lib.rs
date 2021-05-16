use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use gooey_core::{
    euclid::{Point2D, Rect},
    stylecs::Points,
    AnyWidget, Gooey, Renderer, Transmogrifier,
};
use gooey_widgets::button::ButtonTransmogrifier;

mod widgets;

pub struct Rasterizer<R: Renderer> {
    transmogrifiers: HashMap<WidgetTypeId, Box<dyn AnyWidgetRasterizer<R>>>,
    ui: Gooey,
    _phantom: PhantomData<R>,
}

type WidgetTypeId = TypeId;
impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {}

impl<R: Renderer> Rasterizer<R> {
    pub fn new(ui: Gooey) -> Self {
        let mut frontend = Self {
            ui,
            transmogrifiers: HashMap::default(),
            _phantom: PhantomData::default(),
        };

        frontend.register_transmogrifier(ButtonTransmogrifier);

        frontend
    }

    pub fn render(&self, scene: &R) {
        let size = scene.size();

        if let Some(transmogrifier) = self
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

    pub fn register_transmogrifier<M: WidgetRasterizer<R> + 'static>(&mut self, transmogrifier: M) {
        self.transmogrifiers
            .insert(TypeId::of::<M::Widget>(), Box::new(transmogrifier));
    }

    pub fn transmogrifier(
        &self,
        widget_type_id: &TypeId,
    ) -> Option<&'_ dyn AnyWidgetRasterizer<R>> {
        self.transmogrifiers.get(widget_type_id).map(|b| b.as_ref())
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
    T: WidgetRasterizer<R> + 'static,
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
