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

/// A frontend is an implementation of widgets and layouts.
pub trait Frontend: Sized {}

mod gooey;
/// Types used for drawing.
pub mod renderer;
mod widget;

pub use euclid;
pub use stylecs;

pub use self::{gooey::*, renderer::Renderer, widget::*};
