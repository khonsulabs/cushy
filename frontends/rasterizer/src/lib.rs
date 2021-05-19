use std::{any::TypeId, ops::Deref, sync::Arc};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    styles::Points,
    AnySendSync, AnyTransmogrifier, AnyWidgetInstance, Gooey, Transmogrifier, TransmogrifierState,
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

        self.ui
            .with_transmogrifier(self.ui.root_widget(), |transmogrifier, state| {
                transmogrifier.render(
                    state,
                    &Rasterizer {
                        ui: self.ui.clone(),
                        renderer: Some(scene),
                    },
                    self.ui.root_widget(),
                    Rect::new(Point2D::default(), size),
                );
            });
    }
}

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render(
        &self,
        state: &Self::State,
        rasterizer: &Rasterizer<R>,
        widget: &<Self as Transmogrifier<Rasterizer<R>>>::Widget,
        bounds: Rect<f32, Points>,
    );

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn content_size(
        &self,
        state: &Self::State,
        widget: &Self::Widget,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

pub trait AnyWidgetRasterizer<R: Renderer>: Send + Sync {
    fn default_state(&self) -> TransmogrifierState;
    fn widget_type_id(&self) -> TypeId;
    fn render(
        &self,
        state: &mut dyn AnySendSync,
        rasterizer: &Rasterizer<R>,
        widget: &dyn AnyWidgetInstance,
        bounds: Rect<f32, Points>,
    );
    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidgetInstance,
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
        state: &mut dyn AnySendSync,
        rasterizer: &Rasterizer<R>,
        widget: &dyn AnyWidgetInstance,
        bounds: Rect<f32, Points>,
    ) {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<Rasterizer<R>>>::State>()
            .unwrap();
        <T as WidgetRasterizer<R>>::render(&self, state, rasterizer, widget, bounds)
    }

    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidgetInstance,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<Rasterizer<R>>>::State>()
            .unwrap();
        <T as WidgetRasterizer<R>>::content_size(&self, state, widget, rasterizer, constraints)
    }

    fn default_state(&self) -> TransmogrifierState {
        TransmogrifierState(Box::new(
            <<T as Transmogrifier<Rasterizer<R>>>::State as Default>::default(),
        ))
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
        self.0.widget_type_id()
    }

    fn default_state(&self) -> gooey_core::TransmogrifierState {
        self.0.default_state()
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
