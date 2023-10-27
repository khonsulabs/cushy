#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod children;
pub mod context;
pub mod dynamic;
pub mod graphics;
pub mod names;
pub mod styles;
mod tree;
mod utils;
pub mod widget;
pub mod widgets;
pub mod window;

pub use kludgine;
pub use kludgine::app::winit::error::EventLoopError;
pub use kludgine::app::winit::event::ElementState;
use kludgine::figures::units::UPx;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ConstraintLimit {
    Known(UPx),
    ClippedAfter(UPx),
}

impl ConstraintLimit {
    #[must_use]
    pub fn max(self) -> UPx {
        match self {
            ConstraintLimit::Known(v) | ConstraintLimit::ClippedAfter(v) => v,
        }
    }
}

pub type Result<T, E = EventLoopError> = std::result::Result<T, E>;

pub trait Run: Sized {
    fn run(self) -> Result<(), EventLoopError>;
}
