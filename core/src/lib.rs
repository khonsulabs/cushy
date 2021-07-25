//! Core traits and types used to create Graphical User Interfaces (GUIs -
//! `gooey`s).

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    clippy::nursery,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(clippy::if_not_else)]
#![cfg_attr(doc, warn(rustdoc::all))]

mod frontend;
mod gooey;
/// Types used for styling.
pub mod styles;
mod widget;

pub use euclid;
pub use palette;

pub use self::{frontend::*, gooey::*, widget::*};

/// A unit representing physical pixels on a display.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pixels;

/// A unit representing [Desktop publishing points/PostScript points](https://en.wikipedia.org/wiki/Point_(typography)#Desktop_publishing_point). Measurements in this scale are equal to 1/72 of an [imperial inch](https://en.wikipedia.org/wiki/Inch).
#[derive(Debug, Clone, Copy, Default)]
pub struct Points;

/// The name of the class assigned to the root widget of a window.
pub const ROOT_CLASS: &str = "gooey-root";

/// The name of the class assigned to widgets that have a solid background
pub const SOLID_WIDGET_CLASS: &str = "gooey-solid";
