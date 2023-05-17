use gooey_core::Frontend;

#[derive(Debug)]
pub struct Rasterizer;

impl Frontend for Rasterizer {
    type Context = RasterContext;
    type Instance = Rasterized;
}

pub struct Rasterized;

pub struct RasterContext;
