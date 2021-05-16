use std::{any::TypeId, collections::HashMap, marker::PhantomData};

use gooey_core::{
    euclid::{Point2D, Rect},
    stylecs::Points,
    AnyWidget, Gooey, Renderer, Transmogrifier, Widget,
};
use gooey_widgets::button::ButtonTransmogrifier;

mod widgets;

pub struct Rasterized<R: Renderer> {
    transmogrifiers: HashMap<WidgetTypeId, Box<dyn AnyWidgetRasterizer<R>>>,
    ui: Gooey,
    _phantom: PhantomData<R>,
}

type WidgetTypeId = TypeId;
impl<R: Renderer> gooey_core::Frontend for Rasterized<R> {}

impl<R: Renderer> Rasterized<R> {
    pub fn new(ui: Gooey) -> Self {
        let mut frontend = Self {
            ui,
            transmogrifiers: HashMap::default(),
            _phantom: PhantomData::default(),
        };

        frontend.register_transmogrifier(ButtonTransmogrifier);

        frontend
    }

    pub fn update(&mut self) -> bool {
        self.ui.update()
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

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterized<R>> {
    fn render(
        &self,
        scene: &R,
        state: &<<Self as Transmogrifier<Rasterized<R>>>::Widget as Widget>::State,
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
        let state = widget
            .state_as_any()
            .unwrap()
            .downcast_ref::<<<T as Transmogrifier<Rasterized<R>>>::Widget as Widget>::State>()
            .unwrap();
        <T as WidgetRasterizer<R>>::render(&self, scene, state, bounds)
    }
}
