use std::{
    any::TypeId,
    collections::HashMap,
    convert::TryFrom,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    styles::Points,
    AnySendSync, AnyTransmogrifier, AnyWidget, Gooey, Transmogrifier, TransmogrifierState,
    WidgetId, WidgetRegistration,
};
use winit::event::DeviceId;

#[derive(Debug)]
pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    last_raster: Arc<Mutex<RasterResult>>,
    renderer: Option<R>,
}

impl<R: Renderer> Clone for Rasterizer<R> {
    /// This implementation ignores the `renderer` field, as it's temporary
    /// state only used during the render method. It shouldn't ever be accessed
    /// outside of that context.
    fn clone(&self) -> Self {
        Self {
            ui: self.ui.clone(),
            last_raster: Arc::default(),
            renderer: None,
        }
    }
}

impl<R: Renderer> gooey_core::Frontend for Rasterizer<R> {
    type AnyTransmogrifier = RegisteredTransmogrifier<R>;
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
            last_raster: Arc::default(),
            renderer: None,
        }
    }

    pub fn render(&self, scene: R) {
        {
            let mut last_raster = self.last_raster.lock().unwrap();
            last_raster.reset();
        }
        let size = scene.size();

        self.ui.with_transmogrifier(
            self.ui.root_widget().id(),
            self,
            |transmogrifier, state, widget| {
                transmogrifier.render_within(
                    AnyRasterContext::new(
                        self.ui.root_widget().clone(),
                        state,
                        &Rasterizer {
                            ui: self.ui.clone(),
                            last_raster: self.last_raster.clone(),
                            renderer: Some(scene),
                        },
                        widget,
                    ),
                    Rect::new(Point2D::default(), size),
                );
            },
        );
    }

    pub fn clipped_to(&self, clip: Rect<f32, Points>) -> Option<Self> {
        self.renderer().map(|renderer| Self {
            ui: self.ui.clone(),
            last_raster: self.last_raster.clone(),
            renderer: Some(renderer.clip_to(clip)),
        })
    }

    pub fn handle_winit_event<'evt, T>(
        &self,
        scene: R,
        device: &DeviceId,
        event: &winit::event::Event<'evt, T>,
    ) {
        // TODO:
        // * The existing handling of focus/hover seemed fine from memory. The
        //   pain points were around styling, not the actual application of
        //   state. Actually there was a need of refactoring for code-reuse --
        //   each of the mouse event handlers were very similar.
    }

    pub fn renderer(&self) -> Option<&R> {
        self.renderer.as_ref()
    }

    pub fn rasterizerd_widget(&self, widget: WidgetId, bounds: Rect<f32, Points>) {
        let mut raster = self.last_raster.lock().unwrap();
        raster.widget_rendered(widget, bounds);
    }
}

pub struct RasterContext<'a, T: WidgetRasterizer<R>, R: Renderer> {
    pub registration: Arc<WidgetRegistration>,
    pub state: &'a mut <T as Transmogrifier<Rasterizer<R>>>::State,
    pub rasterizer: &'a Rasterizer<R>,
    pub widget: &'a <T as Transmogrifier<Rasterizer<R>>>::Widget,
    _transmogrifier: PhantomData<T>,
}

impl<'a, T: WidgetRasterizer<R>, R: Renderer> RasterContext<'a, T, R> {
    pub fn new(
        registration: Arc<WidgetRegistration>,
        state: &'a mut <T as Transmogrifier<Rasterizer<R>>>::State,
        rasterizer: &'a Rasterizer<R>,
        widget: &'a <T as Transmogrifier<Rasterizer<R>>>::Widget,
    ) -> Self {
        Self {
            registration,
            state,
            rasterizer,
            widget,
            _transmogrifier: PhantomData::default(),
        }
    }
}

impl<'a, T: WidgetRasterizer<R>, R: Renderer> TryFrom<AnyRasterContext<'a, R>>
    for RasterContext<'a, T, R>
{
    type Error = ();

    fn try_from(context: AnyRasterContext<'a, R>) -> Result<Self, Self::Error> {
        let widget = context
            .widget
            .as_any()
            .downcast_ref::<<T as Transmogrifier<Rasterizer<R>>>::Widget>()
            .ok_or(())?;
        let state = context
            .state
            .as_mut_any()
            .downcast_mut::<<T as Transmogrifier<Rasterizer<R>>>::State>()
            .ok_or(())?;
        Ok(RasterContext::new(
            context.registration.clone(),
            state,
            context.rasterizer,
            widget,
        ))
    }
}

pub struct AnyRasterContext<'a, R: Renderer> {
    pub registration: Arc<WidgetRegistration>,
    pub state: &'a mut dyn AnySendSync,
    pub rasterizer: &'a Rasterizer<R>,
    pub widget: &'a dyn AnyWidget,
}

impl<'a, R: Renderer> AnyRasterContext<'a, R> {
    pub fn new(
        registration: Arc<WidgetRegistration>,
        state: &'a mut dyn AnySendSync,
        rasterizer: &'a Rasterizer<R>,
        widget: &'a dyn AnyWidget,
    ) -> Self {
        Self {
            registration,
            state,
            rasterizer,
            widget,
        }
    }
}
pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + Sized + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render_within(&self, context: RasterContext<'_, Self, R>, bounds: Rect<f32, Points>) {
        if let Some(rasterizer) = context.rasterizer.clipped_to(bounds) {
            rasterizer.rasterizerd_widget(
                context.registration.id().clone(),
                rasterizer.renderer().unwrap().clip_bounds(),
            );
            self.render(RasterContext::new(
                context.registration.clone(),
                context.state,
                &rasterizer,
                context.widget,
            ));
        }
    }

    fn render(&self, context: RasterContext<'_, Self, R>);

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn content_size(
        &self,
        context: RasterContext<'_, Self, R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

pub trait AnyWidgetRasterizer<R: Renderer>: AnyTransmogrifier<Rasterizer<R>> + Send + Sync {
    fn render_within(&self, context: AnyRasterContext<'_, R>, bounds: Rect<f32, Points>);
    fn content_size(
        &self,
        context: AnyRasterContext<'_, R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + AnyTransmogrifier<Rasterizer<R>> + Send + Sync + 'static,
    R: Renderer,
{
    fn render_within(&self, context: AnyRasterContext<'_, R>, bounds: Rect<f32, Points>) {
        <T as WidgetRasterizer<R>>::render_within(
            &self,
            RasterContext::try_from(context).unwrap(),
            bounds,
        )
    }

    fn content_size(
        &self,
        context: AnyRasterContext<'_, R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        <T as WidgetRasterizer<R>>::content_size(
            &self,
            RasterContext::try_from(context).unwrap(),
            constraints,
        )
    }
}

impl<R: Renderer> AnyTransmogrifier<Rasterizer<R>> for RegisteredTransmogrifier<R> {
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

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &Arc<WidgetRegistration>,
        frontend: &Rasterizer<R>,
    ) -> TransmogrifierState {
        self.0.default_state_for(widget, registration, frontend)
    }
}

#[derive(Debug)]
pub struct RegisteredTransmogrifier<R: Renderer>(pub Box<dyn AnyWidgetRasterizer<R>>);

impl<R: Renderer> Deref for RegisteredTransmogrifier<R> {
    type Target = Box<dyn AnyWidgetRasterizer<R>>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
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

#[derive(Default, Debug)]
struct RasterResult {
    pub order: Vec<WidgetId>,
    pub bounds: HashMap<u32, Rect<f32, Points>>,
}

impl RasterResult {
    pub fn reset(&mut self) {
        self.order.clear();
        self.bounds.clear();
    }

    pub fn widget_rendered(&mut self, widget: WidgetId, bounds: Rect<f32, Points>) {
        self.bounds.insert(widget.id, bounds);
        self.order.push(widget);
    }
}
