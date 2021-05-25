use std::{convert::TryFrom, marker::PhantomData, sync::Arc};

use gooey_core::{renderer::Renderer, AnySendSync, AnyWidget, Transmogrifier, WidgetRegistration};

use crate::{Rasterizer, WidgetRasterizer};

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
