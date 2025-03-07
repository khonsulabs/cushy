#![doc = include_str!("../.crate-docs.md")]
#![warn(clippy::pedantic, missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::doc_lazy_continuation
)]
// Recursion limit setting is necessary after updating from wgpu 23.0.0 to
// 24.0.0 caused "error[E0275]: overflow evaluating the requirement
// `NumericType: Sync`".
#![recursion_limit = "256"]

// for proc-macros
extern crate core;
extern crate self as cushy;

#[macro_use]
mod utils;

pub mod animation;
pub mod context;
pub mod graphics;
mod names;
#[macro_use]
pub mod styles;
mod app;
pub mod debug;
pub mod fonts;
pub mod reactive;
mod tick;
mod tree;
pub mod widget;
pub mod widgets;
pub mod window;

pub mod dialog;

#[doc(hidden)]
pub mod example;
#[cfg(feature = "localization")]
#[macro_use]
pub mod localization;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A string that may be a localized message.
#[derive(Clone, Debug)]
pub enum MaybeLocalized {
    /// A non-localized message.
    Text(String),
    #[cfg(feature = "localization")]
    /// A localized message.
    Localized(localization::Localize),
}

impl Default for MaybeLocalized {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

impl From<&str> for MaybeLocalized {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for MaybeLocalized {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl IntoValue<MaybeLocalized> for &str {
    fn into_value(self) -> Value<MaybeLocalized> {
        Value::Constant(MaybeLocalized::from(self))
    }
}

impl IntoValue<MaybeLocalized> for String {
    fn into_value(self) -> Value<MaybeLocalized> {
        Value::Constant(MaybeLocalized::from(self))
    }
}
impl MaybeLocalized {
    #[cfg_attr(not(feature = "localization"), allow(unused_variables))]
    fn localize_for_cushy(&self, app: &Cushy) -> String {
        match self {
            MaybeLocalized::Text(text) => text.clone(),
            #[cfg(feature = "localization")]
            MaybeLocalized::Localized(localized) => localized.localize(
                &localization::WindowTranslationContext(&app.data.localizations),
            ),
        }
    }
}

#[cfg(not(feature = "localization"))]
impl widgets::label::DynamicDisplay for MaybeLocalized {
    fn fmt(
        &self,
        _context: &context::WidgetContext<'_>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            MaybeLocalized::Text(text) => std::fmt::Display::fmt(text, f),
        }
    }
}

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
/// continue running until the last window is closed. **Local variables in the
/// function will be dropped before the application runs.**
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
/// # fn main() {
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
/// # fn main() {
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
/// # fn main() {
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
/// # fn main() {
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
use figures::{IntoUnsigned, Size, Zero};
use kludgine::app::winit::error::EventLoopError;
pub use names::Name;
use reactive::value::{IntoValue, Value};
pub use utils::{Lazy, ModifiersExt, ModifiersStateExt, WithClone};
pub use {figures, kludgine};

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
    pub fn fit_measured<Unit>(self, measured: Unit) -> UPx
    where
        Unit: IntoUnsigned<Unsigned = UPx>,
    {
        match self {
            ConstraintLimit::Fill(size) => size.max(measured.into_unsigned()),
            ConstraintLimit::SizeToFit(_) => measured.into_unsigned(),
        }
    }

    /// When `self` is `SizeToFit`, the smallest of the constraint and
    /// `measured` will be returned. When `self` is `Fill`, the fill size will
    /// be returned.
    pub fn fill_or_fit<Unit>(self, measured: Unit) -> UPx
    where
        Unit: IntoUnsigned<Unsigned = UPx>,
    {
        match self {
            ConstraintLimit::Fill(size) => size,
            ConstraintLimit::SizeToFit(size) => size.min(measured.into_unsigned()),
        }
    }
}

/// An extension trait for `Size<ConstraintLimit>`.
pub trait FitMeasuredSize {
    /// Returns the result of calling [`ConstraintLimit::fit_measured`] for each
    /// matching component in `self` and `measured`.
    fn fit_measured<Unit>(self, measured: Size<Unit>) -> Size<UPx>
    where
        Unit: IntoUnsigned<Unsigned = UPx>;
}

impl FitMeasuredSize for Size<ConstraintLimit> {
    fn fit_measured<Unit>(self, measured: Size<Unit>) -> Size<UPx>
    where
        Unit: IntoUnsigned<Unsigned = UPx>,
    {
        Size::new(
            self.width.fit_measured(measured.width),
            self.height.fit_measured(measured.height),
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
        use tracing_subscriber::filter::{LevelFilter, Targets};
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
            .with(
                Targets::new()
                    .with_default(MAX_LEVEL)
                    .with_target("winit", Level::ERROR)
                    .with_target("wgpu", Level::ERROR)
                    .with_target("naga", Level::ERROR)
                    .with_default(MAX_LEVEL),
            )
            .try_init();
    }
}
