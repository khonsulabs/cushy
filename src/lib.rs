#![doc = include_str!("../.crate-docs.md")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::doc_lazy_continuation
)]

// for proc-macros
extern crate self as cushy;

#[macro_use]
mod utils;

pub mod animation;
pub mod context;
mod graphics;
mod names;
#[macro_use]
pub mod styles;
mod app;
pub mod debug;
pub mod fonts;
mod tick;
mod tree;
pub mod value;
pub mod widget;
pub mod widgets;
pub mod window;

#[doc(hidden)]
pub mod example;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(feature = "tokio")]
pub use app::TokioRuntime;
pub use app::{
    App, AppRuntime, Application, Cushy, DefaultRuntime, Open, PendingApp, Run, ShutdownGuard,
};
/// A macro to create a `main()` function with less boilerplate.
///
/// When creating applications that support multiple windows, this attribute
/// macro can be used to remove a few lines of code.
///
/// The function body is executed during application startup, and the app will
/// continue running until the last window is closed.
///
/// This attribute must be attached to a `main(&mut PendingApp)` or `main(&mut
/// App)` function. Either form supports a return type or no return type.
///
/// ## `&mut PendingApp`
///
/// When using a [`PendingApp`], the function body is invoked before the app is
/// run. While the example shown below does not require the runtime
/// initialization, some programs do and using the macro means the developer
/// will never forget to add the extra code.
///
/// These two example programs are functionally identical:
///
/// ### Without Macro
///
/// ```rust
/// # fn test() {
/// use cushy::{Open, PendingApp, Run};
///
/// fn main() -> cushy::Result {
///     let mut app = PendingApp::default();
///     let cushy = app.cushy().clone();
///     let _guard = cushy.enter_runtime();
///
///     "Hello World".open(&mut app)?;
///
///     app.run()
/// }
/// # }
/// ```
///
/// ### With Macro
///
/// ```rust
/// # fn test() {
/// use cushy::{Open, PendingApp};
///
/// #[cushy::main]
/// fn main(app: &mut PendingApp) -> cushy::Result {
///     "Hello World".open(app)?;
///     Ok(())
/// }
/// # }
/// ```
///
/// ## `&mut App`
///
/// When using an [`App`], the function body is invoked after the app's event
/// loop has begun executing. This is important if the application wants to
/// access monitor information to either position windows precisely or use a
/// full screen video mode.
///
/// These two example programs are functionally identical:
///
/// ### Without Macro
///
/// ```rust
/// # fn test() {
/// use cushy::{App, Open, PendingApp, Run};
///
/// fn main() -> cushy::Result {
///     let mut app = PendingApp::default();
///     app.on_startup(|app| -> cushy::Result {
///         "Hello World".open(app)?;
///         Ok(())
///     });
///     app.run()
/// }
/// # }
/// ```
///
/// ### With Macro
///
/// ```rust
/// # fn test() {
/// use cushy::{App, Open};
///
/// #[cushy::main]
/// fn main(app: &mut App) -> cushy::Result {
///     "Hello World".open(app)?;
///     Ok(())
/// }
/// # }
/// ```
pub use cushy_macros::main;
use figures::units::UPx;
use figures::{Fraction, ScreenUnit, Size, Zero};
use kludgine::app::winit::error::EventLoopError;
pub use names::Name;
pub use utils::{Lazy, ModifiersExt, ModifiersStateExt, WithClone};
pub use {figures, kludgine};

pub use self::graphics::Graphics;
pub use self::tick::{InputState, Tick};

/// Starts running a Cushy application, invoking `app_init` after the event loop
/// has started.
pub fn run<F>(app_init: F) -> Result
where
    F: FnOnce(&mut App) + Send + 'static,
{
    let mut app = PendingApp::default();
    app.on_startup(app_init);
    app.run()
}

/// A limit used when measuring a widget.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConstraintLimit {
    /// The widget is expected to occupy a known size.
    Fill(UPx),
    /// The widget is expected to resize itself to fit its contents, trying to
    /// stay within the size given.
    SizeToFit(UPx),
}

impl ConstraintLimit {
    /// Returns `UPx::ZERO` when sizing to fit, otherwise it returns the size
    /// being filled.
    #[must_use]
    pub fn min(self) -> UPx {
        match self {
            ConstraintLimit::Fill(v) => v,
            ConstraintLimit::SizeToFit(_) => UPx::ZERO,
        }
    }

    /// Returns the maximum measurement that will fit the constraint.
    #[must_use]
    pub fn max(self) -> UPx {
        match self {
            ConstraintLimit::Fill(v) | ConstraintLimit::SizeToFit(v) => v,
        }
    }

    /// Converts `measured` to unsigned pixels, and adjusts it according to the
    /// constraint's intentions.
    ///
    /// If this constraint is of a known size, it will return the maximum of the
    /// measured size and the constraint. If it is of an unknown size, it will
    /// return the measured size.
    pub fn fit_measured<Unit>(self, measured: Unit, scale: Fraction) -> UPx
    where
        Unit: ScreenUnit,
    {
        let measured = measured.into_upx(scale);
        match self {
            ConstraintLimit::Fill(size) => size.max(measured),
            ConstraintLimit::SizeToFit(_) => measured,
        }
    }
}

/// An extension trait for `Size<ConstraintLimit>`.
pub trait FitMeasuredSize {
    /// Returns the result of calling [`ConstraintLimit::fit_measured`] for each
    /// matching component in `self` and `measured`.
    fn fit_measured<Unit>(self, measured: Size<Unit>, scale: Fraction) -> Size<UPx>
    where
        Unit: ScreenUnit;
}

impl FitMeasuredSize for Size<ConstraintLimit> {
    fn fit_measured<Unit>(self, measured: Size<Unit>, scale: Fraction) -> Size<UPx>
    where
        Unit: ScreenUnit,
    {
        Size::new(
            self.width.fit_measured(measured.width, scale),
            self.height.fit_measured(measured.height, scale),
        )
    }
}

impl Add<UPx> for ConstraintLimit {
    type Output = Self;

    fn add(mut self, rhs: UPx) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<UPx> for ConstraintLimit {
    fn add_assign(&mut self, rhs: UPx) {
        *self = match *self {
            ConstraintLimit::Fill(px) => ConstraintLimit::Fill(px.saturating_add(rhs)),
            ConstraintLimit::SizeToFit(px) => ConstraintLimit::SizeToFit(px.saturating_add(rhs)),
        };
    }
}

impl Sub<UPx> for ConstraintLimit {
    type Output = Self;

    fn sub(mut self, rhs: UPx) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign<UPx> for ConstraintLimit {
    fn sub_assign(&mut self, rhs: UPx) {
        *self = match *self {
            ConstraintLimit::Fill(px) => ConstraintLimit::Fill(px.saturating_sub(rhs)),
            ConstraintLimit::SizeToFit(px) => ConstraintLimit::SizeToFit(px.saturating_sub(rhs)),
        };
    }
}

/// A result alias that defaults to the result type commonly used throughout
/// this crate.
pub type Result<T = (), E = EventLoopError> = std::result::Result<T, E>;

/// Counts the number of expressions passed to it.
///
/// This is used inside of Cushy macros to preallocate collections.
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
