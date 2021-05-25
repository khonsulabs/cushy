use std::{any::TypeId, ops::Deref, sync::Arc};

#[doc(hidden)]
pub use gooey_core::renderer::Renderer;
use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    styles::Points,
    AnySendSync, AnyTransmogrifier, AnyWidget, Gooey, Transmogrifier, TransmogrifierState,
    WidgetRegistration,
};
use winit::event::DeviceId;

#[derive(Debug)]
pub struct Rasterizer<R: Renderer> {
    pub ui: Arc<Gooey<Self>>,
    renderer: Option<R>,
}

impl<R: Renderer> Clone for Rasterizer<R> {
    /// This implementation ignores the `renderer` field, as it's temporary
    /// state only used during the render method. It shouldn't ever be accessed
    /// outside of that context.
    fn clone(&self) -> Self {
        Self {
            ui: self.ui.clone(),
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
            renderer: None,
        }
    }

    pub fn render(&self, scene: R) {
        let size = scene.size();

        self.ui.with_transmogrifier(
            self.ui.root_widget().id(),
            self,
            |transmogrifier, state, widget| {
                transmogrifier.render_within(
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

    pub fn clipped_to(&self, clip: Rect<f32, Points>) -> Option<Self> {
        self.renderer().map(|renderer| Self {
            ui: self.ui.clone(),
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
        // * Need a list of all widgets that have been rendered.
        // * I removed the ability to get the widgetid from the callbacks by
        //   removing the Channels parameter. We need the widget id back, it
        //   seems like it's a good time to introduce a Context parameter that
        //   can house the common paramters to all transmogrifier callbacks --
        //   state, widget id/reg, frontend, widget.
        // * Once we have the widget id, we can have a simple Mutex<collection>
        //   that gets cleared on render start. During `render_within` we can
        //   note the id and location into the lookup structure. Order is
        //   important, but it also seems like a spatial map would be nice...
        //   Vec is easiest and likely will be fine for any sane application
        //   (ie, not oodles of widgets).
        // * The existing handling of focus/hover seemed fine from memory. The
        //   pain points were around styling, not the actual application of
        //   state. Actually there was a need of refactoring for code-reuse --
        //   each of the mouse event handlers were very similar.
    }

    pub fn renderer(&self) -> Option<&R> {
        self.renderer.as_ref()
    }
}

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render_within(
        &self,
        state: &Self::State,
        rasterizer: &Rasterizer<R>,
        widget: &<Self as Transmogrifier<Rasterizer<R>>>::Widget,
        bounds: Rect<f32, Points>,
    ) {
        if let Some(rasterizer) = rasterizer.clipped_to(bounds) {
            self.render(state, &rasterizer, widget);
            // TODO notate that it's rendered.
        }
    }

    fn render(
        &self,
        state: &Self::State,
        rasterizer: &Rasterizer<R>,
        widget: &<Self as Transmogrifier<Rasterizer<R>>>::Widget,
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

pub trait AnyWidgetRasterizer<R: Renderer>: AnyTransmogrifier<Rasterizer<R>> + Send + Sync {
    fn render_within(
        &self,
        state: &mut dyn AnySendSync,
        rasterizer: &Rasterizer<R>,
        widget: &dyn AnyWidget,
        bounds: Rect<f32, Points>,
    );
    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
        rasterizer: &Rasterizer<R>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + AnyTransmogrifier<Rasterizer<R>> + Send + Sync + 'static,
    R: Renderer,
{
    fn render_within(
        &self,
        state: &mut dyn AnySendSync,
        rasterizer: &Rasterizer<R>,
        widget: &dyn AnyWidget,
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
        <T as WidgetRasterizer<R>>::render_within(&self, state, rasterizer, widget, bounds)
    }

    fn content_size(
        &self,
        state: &mut dyn AnySendSync,
        widget: &dyn AnyWidget,
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
