use std::{any::TypeId, convert::TryFrom, ops::Deref, sync::Arc};

use gooey_core::{
    euclid::{Rect, Size2D},
    renderer::Renderer,
    styles::Points,
    AnySendSync, AnyTransmogrifier, AnyWidget, Transmogrifier, TransmogrifierState,
    WidgetRegistration,
};

use crate::{AnyRasterContext, RasterContext, Rasterizer};

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
