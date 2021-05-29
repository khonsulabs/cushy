use std::{convert::TryFrom, marker::PhantomData};

use gooey_core::{
    renderer::Renderer,
    styles::{style_sheet::State, Style},
    AnySendSync, AnyWidget, Transmogrifier, WidgetRegistration,
};

use crate::{Rasterizer, WidgetRasterizer};

pub struct RasterContext<'a, T: WidgetRasterizer<R>, R: Renderer> {
    pub registration: WidgetRegistration,
    pub state: &'a mut <T as Transmogrifier<Rasterizer<R>>>::State,
    pub rasterizer: &'a Rasterizer<R>,
    pub widget: &'a <T as Transmogrifier<Rasterizer<R>>>::Widget,
    pub style: &'a Style,
    pub ui_state: &'a State,
    _transmogrifier: PhantomData<T>,
}

impl<'a, T: WidgetRasterizer<R>, R: Renderer> RasterContext<'a, T, R> {
    pub fn new(
        registration: WidgetRegistration,
        state: &'a mut <T as Transmogrifier<Rasterizer<R>>>::State,
        rasterizer: &'a Rasterizer<R>,
        widget: &'a <T as Transmogrifier<Rasterizer<R>>>::Widget,
        style: &'a Style,
        ui_state: &'a State,
    ) -> Self {
        Self {
            registration,
            state,
            rasterizer,
            widget,
            style,
            ui_state,
            _transmogrifier: PhantomData::default(),
        }
    }
}

impl<'a, 'b, T: WidgetRasterizer<R>, R: Renderer> TryFrom<&'b mut AnyRasterContext<'a, R>>
    for RasterContext<'b, T, R>
{
    type Error = ();

    fn try_from(context: &'b mut AnyRasterContext<'a, R>) -> Result<Self, Self::Error> {
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
            context.style,
            context.ui_state,
        ))
    }
}

pub struct AnyRasterContext<'a, R: Renderer> {
    pub registration: WidgetRegistration,
    pub state: &'a mut dyn AnySendSync,
    pub rasterizer: &'a Rasterizer<R>,
    pub widget: &'a dyn AnyWidget,
    pub style: &'a Style,
    pub ui_state: &'a State,
}

impl<'a, R: Renderer> AnyRasterContext<'a, R> {
    pub fn new(
        registration: WidgetRegistration,
        state: &'a mut dyn AnySendSync,
        rasterizer: &'a Rasterizer<R>,
        widget: &'a dyn AnyWidget,
        style: &'a Style,
        ui_state: &'a State,
    ) -> Self {
        Self {
            registration,
            state,
            rasterizer,
            widget,
            style,
            ui_state,
        }
    }
}
