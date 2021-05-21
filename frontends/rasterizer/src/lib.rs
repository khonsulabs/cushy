use std::{any::TypeId, ops::Deref, sync::Arc};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    styles::Points,
    AnyFrontendTransmogrifier, AnySendSync, AnyWidget, Frontend, Gooey, Transmogrifier,
    TransmogrifierState, WidgetId,
};

pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    renderer: Option<R>,
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyWidgetTransmogrifier = RegisteredTransmogrifier<Self>;
    type Context = Self;

    fn gooey(&self) -> &'_ Gooey<Self> {
        &self.ui
    }
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

        self.ui.with_transmogrifier(
            self.ui.root_widget().id(),
            |transmogrifier, state, widget, channels| {
                transmogrifier.render(
                    state,
                    &Rasterizer {
                        ui: self.ui.clone(),
                        renderer: Some(scene),
                    },
                    widget,
                    Rect::new(Point2D::default(), size),
                );
            },
        );
    }
}

pub trait WidgetRasterizer<F: Frontend>: Transmogrifier<F> + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<F>>::Widget>()
    }

    fn render(
        &self,
        state: &Self::State,
        rasterizer: &F,
        widget: &<Self as Transmogrifier<F>>::Widget,
        bounds: Rect<f32, Points>,
    );

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn content_size(
        &self,
        state: &Self::State,
        widget: &Self::Widget,
        rasterizer: &F,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

pub trait AnyWidgetRasterizer<F: Frontend>: AnyFrontendTransmogrifier<F> + Send + Sync {
    fn render(
        &self,
        state: &mut dyn AnySendSync,
        rasterizer: &F,
        widget: &dyn AnyWidget,
        bounds: Rect<f32, Points>,
    );
    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        rasterizer: &F,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

impl<T, F> AnyWidgetRasterizer<F> for T
where
    T: WidgetRasterizer<F> + AnyFrontendTransmogrifier<F> + Send + Sync + 'static,
    F: Frontend,
{
    fn render(
        &self,
        state: &mut dyn AnySendSync,
        rasterizer: &F,
        widget: &dyn AnyWidget,
        bounds: Rect<f32, Points>,
    ) {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<F>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<F>>::State>()
            .unwrap();
        <T as WidgetRasterizer<F>>::render(&self, state, rasterizer, widget, bounds)
    }

    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        rasterizer: &F,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        let widget = widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<F>>::Widget>()
            .unwrap();
        let state = state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<F>>::State>()
            .unwrap();
        <T as WidgetRasterizer<F>>::content_size(&self, state, widget, rasterizer, constraints)
    }
}

impl<R: Renderer> AnyFrontendTransmogrifier<Rasterizer<R>>
    for RegisteredTransmogrifier<Rasterizer<R>>
{
    fn process_messages(
        &self,
        state: &mut dyn AnySendSync,
        widget: &mut dyn AnyWidget,
        channels: &dyn gooey_core::AnyChannels,
        frontend: &Rasterizer<R>,
    ) {
        self.0
            .as_ref()
            .process_messages(state, widget, channels, frontend);
    }

    fn widget_type_id(&self) -> TypeId {
        self.0.widget_type_id()
    }

    fn default_state_for(&self, widget_id: WidgetId) -> TransmogrifierState {
        self.0.default_state_for(widget_id)
    }
}

pub struct RegisteredTransmogrifier<F: Frontend>(pub Box<dyn AnyWidgetRasterizer<F>>);

impl<F: Frontend> Deref for RegisteredTransmogrifier<F> {
    type Target = Box<dyn AnyWidgetRasterizer<F>>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
    }
}

#[macro_export]
macro_rules! make_rasterized {
    ($transmogrifier:ident) => {
        impl<R: $crate::Renderer> From<$transmogrifier>
            for $crate::RegisteredTransmogrifier<$crate::Rasterizer<R>>
        {
            fn from(transmogrifier: $transmogrifier) -> Self {
                Self(std::boxed::Box::new(transmogrifier))
            }
        }
    };
}
