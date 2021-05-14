use std::{any::TypeId, collections::HashMap};

use gooey_core::{AnyWidget, Gooey, Materializer, Widget};
use gooey_widgets::button::ButtonMaterializer;
use kludgine::prelude::*;

mod widgets;

pub struct Kludgine {
    materializers: HashMap<WidgetTypeId, Box<dyn AnyRenderer>>,
    ui: Gooey,
}

type WidgetTypeId = TypeId;
impl gooey_core::Frontend for Kludgine {}

impl Kludgine {
    pub fn new(ui: Gooey) -> Self {
        let mut frontend = Self {
            ui,
            materializers: HashMap::default(),
        };

        frontend.register_materializer(ButtonMaterializer);

        frontend
    }

    pub fn update(&mut self) -> bool {
        self.ui.update()
    }

    pub fn render(&self, scene: &Target<'_>) {
        let size = scene.size();
        let children = self.ui.layout_within(size.cast_unit());

        if let Some(materializer) = self.root_materializer() {
            materializer.render(
                scene,
                self.ui.root_widget(),
                Rect::new(Point::default(), size),
            );
        }

        if !children.is_empty() {
            todo!()
        }
    }

    pub fn register_materializer<M: KludgineRenderer + 'static>(&mut self, materializer: M) {
        self.materializers
            .insert(TypeId::of::<M::Widget>(), Box::new(materializer));
    }

    pub fn materializer(&self, widget_type_id: &TypeId) -> Option<&'_ dyn AnyRenderer> {
        self.materializers.get(widget_type_id).map(|b| b.as_ref())
    }

    pub fn root_materializer(&self) -> Option<&'_ dyn AnyRenderer> {
        self.materializer(&self.ui.root_widget().widget_type_id())
    }
}

pub trait KludgineRenderer: Materializer<Kludgine> {
    fn render(
        &self,
        scene: &Target,
        state: &<<Self as Materializer<Kludgine>>::Widget as Widget>::State,
        bounds: Rect<f32, Scaled>,
    );
}

pub trait AnyRenderer: Send + Sync + 'static {
    fn render(&self, scene: &Target, widget: &dyn AnyWidget, bounds: Rect<f32, Scaled>);
}

impl<T> AnyRenderer for T
where
    T: KludgineRenderer + 'static,
{
    fn render(&self, scene: &Target, widget: &dyn AnyWidget, bounds: Rect<f32, Scaled>) {
        let state = widget
            .state_as_any()
            .unwrap()
            .downcast_ref::<<<T as Materializer<Kludgine>>::Widget as Widget>::State>()
            .unwrap();
        <T as KludgineRenderer>::render(&self, scene, state, bounds)
    }
}
