use gooey_core::ActiveContext;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn run<Widget, Initializer>(
    widgets: gooey_core::Widgets<gooey_web::WebApp>,
    init: Initializer,
) -> !
where
    Initializer: FnOnce(&ActiveContext) -> Widget,
    Widget: gooey_core::Widget,
{
    gooey_web::attach_to_body(widgets, init);

    wasm_bindgen::throw_str("This is not an actual error. Please ignore.");
}
#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
pub fn run<Widget, Initializer>(
    widgets: gooey_core::Widgets<gooey_raster::Rasterizer>,
    init: Initializer,
) -> !
where
    Initializer: FnOnce(&ActiveContext) -> Widget,
    Widget: gooey_core::Widget,
{
    todo!("need to run the rasterizer")
}

#[cfg(feature = "desktop")]
pub use gooey_raster as raster;
#[cfg(feature = "web")]
pub use gooey_web as web;
pub use {gooey_core as core, gooey_widgets as widgets};
