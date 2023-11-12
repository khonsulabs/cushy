#![doc = include_str!("../.crate-docs.md")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

#[macro_use]
mod utils;

pub mod animation;
pub mod context;
mod graphics;
mod names;
#[macro_use]
pub mod styles;
mod tick;
mod tree;
pub mod value;
pub mod widget;
pub mod widgets;
pub mod window;
use std::ops::Sub;

pub use kludgine;
use kludgine::app::winit::error::EventLoopError;
use kludgine::figures::units::UPx;
use kludgine::figures::{Fraction, IntoUnsigned, ScreenUnit};
pub use names::Name;
pub use utils::WithClone;

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

    /// Converts `measured` to unsigned pixels, and adjusts it according to the
    /// contraint's intentions.
    ///
    /// If this constraint is of a known size, it will return the maximum of the
    /// measured size and the contraint. If it is of an unknown size, it will
    /// return the measured size.
    pub fn fit_measured<Unit>(self, measured: Unit, scale: Fraction) -> UPx
    where
        Unit: ScreenUnit,
    {
        let measured = measured.into_px(scale).into_unsigned();
        match self {
            ConstraintLimit::Known(size) => size.max(measured),
            ConstraintLimit::ClippedAfter(_) => measured,
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

/// Creates a [`Children`](crate::widget::Children) instance with the given list
/// of widgets.
#[macro_export]
#[deprecated = "use MakeWidget.and()/Children.and() to chain widgets without a macro"]
macro_rules! children {
    () => {
        $crate::widget::Children::new()
    };
    ($($widget:expr),+) => {{
        let mut widgets = $crate::widget::Children::with_capacity($crate::count!($($widget),+ ;));
        $(widgets.push($widget);)+
        widgets
    }};
    ($($widget:expr),+ ,) => {{
        $crate::children!($($widget),+)
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

fn initialize_tracing() {
    #[cfg(feature = "tracing-output")]
    {
        use tracing::Level;
        use tracing_subscriber::filter::LevelFilter;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        use tracing_subscriber::EnvFilter;

        #[cfg(debug_assertions)]
        const MAX_LEVEL: Level = Level::INFO;
        #[cfg(not(debug_assertions))]
        const MAX_LEVEL: Level = Level::ERROR;

        let _result = tracing_subscriber::fmt::fmt()
            .with_max_level(MAX_LEVEL)
            .finish()
            .with(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::from_level(MAX_LEVEL).into())
                    .from_env_lossy(),
            )
            .try_init();
    }
}
