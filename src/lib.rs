#![doc = include_str!("../.crate-docs.md")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

#[macro_use]
mod utils;

pub mod animation;
pub mod context;
mod graphics;
mod names;
pub mod styles;
mod tick;
mod tree;
pub mod value;
pub mod widget;
pub mod widgets;
pub mod window;
use std::ops::Sub;

pub use with_clone::WithClone;
mod with_clone;

pub use kludgine;
use kludgine::app::winit::error::EventLoopError;
use kludgine::figures::units::UPx;
pub use names::Name;

pub use self::graphics::Graphics;
pub use self::tick::{InputState, Tick};

/// A limit used when measuring a widget.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConstraintLimit {
    /// The widget is expected to occupy a known size.
    Known(UPx),
    /// The widget is expected to resize itself to fit within the size provided.
    ClippedAfter(UPx),
}

impl ConstraintLimit {
    /// Returns the maximum measurement that will fit the constraint.
    #[must_use]
    pub fn max(self) -> UPx {
        match self {
            ConstraintLimit::Known(v) | ConstraintLimit::ClippedAfter(v) => v,
        }
    }
}

impl Sub<UPx> for ConstraintLimit {
    type Output = Self;

    fn sub(self, rhs: UPx) -> Self::Output {
        match self {
            ConstraintLimit::Known(px) => ConstraintLimit::Known(px.saturating_sub(rhs)),
            ConstraintLimit::ClippedAfter(px) => {
                ConstraintLimit::ClippedAfter(px.saturating_sub(rhs))
            }
        }
    }
}

/// A result alias that defaults to the result type commonly used throughout
/// this crate.
pub type Result<T = (), E = EventLoopError> = std::result::Result<T, E>;

/// A type that can be run as an application.
pub trait Run: Sized {
    /// Runs the provided type, returning `Ok(())` upon successful execution and
    /// program exit. Note that this function may not ever return on some
    /// platforms.
    fn run(self) -> crate::Result;
}

/// Creates a [`Widgets`](crate::widget::Widgets) instance with the given list
/// of widgets.
#[macro_export]
macro_rules! widgets {
    () => {
        $crate::widget::Widgets::new()
    };
    ($($widget:expr),+) => {{
        let mut widgets = $crate::widget::Widgets::with_capacity($crate::count!($($widget),+ ;));
        $(widgets.push($widget);)+
        widgets
    }};
    ($($widget:expr),+ ,) => {{
        $crate::widgets!($($widget),+)
    }};
}

/// Counts the number of expressions passed to it.
///
/// This is used inside of Gooey macros to preallocate collections.
#[macro_export]
#[doc(hidden)]
macro_rules! count {
    ($value:expr ;) => {
        1
    };
    ($value:expr , $($remaining:expr),+ ;) => {
        1 + $crate::count!($($remaining),+ ;)
    }
}

/// Creates a [`Styles`](crate::styles::Styles) instance with the given
/// name/component pairs.
#[macro_export]
macro_rules! styles {
    () => {{
        $crate::styles::Styles::new()
    }};
    ($($component:expr => $value:expr),*) => {{
        let mut styles = $crate::styles::Styles::with_capacity($crate::count!($($value),* ;));
        $(styles.insert(&$component, $value);)*
        styles
    }};
    ($($component:expr => $value:expr),* ,) => {{
        $crate::styles!($($component => $value),*)
    }};
}
