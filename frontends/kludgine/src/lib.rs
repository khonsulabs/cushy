use std::{any::TypeId, collections::HashMap};

use gooey_core::{AnyWidget, Gooey, Materializer, Widget};
use kludgine::prelude::*;

pub struct Kludgine {
    materializers: HashMap<WidgetTypeId, Box<dyn AnyRenderer>>,
    ui: Gooey,
}

type WidgetTypeId = TypeId;
impl gooey_core::Frontend for Kludgine {}

impl Kludgine {
    pub fn new(ui: Gooey) -> Self {
        Self {
            ui,
            materializers: HashMap::default(),
        }
    }

    pub fn update(&mut self) -> bool {
        self.ui.update()
    }

    pub async fn render(&self, scene: &Target) {
        let size = scene.size().await;
        let children = self.ui.layout_within(size.cast_unit());
        if !children.is_empty() {
            todo!()
        }

        if let Some(materializer) = self.root_materializer() {
            materializer
                .render(
                    scene,
                    self.ui.root_widget(),
                    Rect::new(Point::default(), size),
                )
                .await;
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

#[async_trait]
pub trait KludgineRenderer: Materializer<Kludgine> {
    async fn render(
        &self,
        scene: &Target,
        state: &<<Self as Materializer<Kludgine>>::Widget as Widget>::State,
        bounds: Rect<f32, Scaled>,
    );
}

#[async_trait]
pub trait AnyRenderer: Send + Sync + 'static {
    async fn render(&self, scene: &Target, widget: &dyn AnyWidget, bounds: Rect<f32, Scaled>);
}

#[async_trait]
impl<T> AnyRenderer for T
where
    T: KludgineRenderer + 'static,
{
    async fn render(&self, scene: &Target, widget: &dyn AnyWidget, bounds: Rect<f32, Scaled>) {
        let state = widget
            .state_as_any()
            .unwrap()
            .downcast_ref::<<<T as Materializer<Kludgine>>::Widget as Widget>::State>()
            .unwrap();
        <T as KludgineRenderer>::render(&self, scene, state, bounds).await
    }
}
