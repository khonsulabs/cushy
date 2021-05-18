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
#![cfg_attr(doc, deny(rustdoc))]

mod frontend;
mod gooey;
/// Types used for drawing.
pub mod renderer;
/// Types used for styling.
pub mod styles;
mod widget;

pub use euclid;

pub use self::{frontend::*, gooey::*, widget::*};
