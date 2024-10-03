use std::marker::PhantomData;
use std::process::exit;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use kludgine::app::winit::error::EventLoopError;
use kludgine::app::{AppEvent, AsApplication, ExecutingApp, Monitors};
use parking_lot::{Mutex, MutexGuard};

use crate::fonts::FontCollection;
use crate::window::sealed::WindowCommand;
use crate::window::WindowHandle;
use crate::{animation, initialize_tracing};

/// A Cushy application that has not started running yet.
///
/// ## Logging/Tracing in Cushy
///
/// This type is responsible for initializing Cushy's built-in support for
/// listening for tracing/log messages by installing a global
/// `tracing_subcriber` Subscriber.
///
/// ### To enable logging/tracing support
///
/// Most ways of running a Cushy app will automatically intialize logging
/// because at some point they call `PendingApp::default()`. The default
/// behavior is to initialize logging.
///
/// When using [`PendingApp::new`] to provide a custom [`AppRuntime`], support
/// can be enabled using:
///
/// - [`with_tracing()`](Self::with_tracing)
/// - [`initialize_tracing()`](Self::initialize_tracing)
///
/// ### Overriding Cushy's logging/tracing support
///
/// Cushy uses `tracing_subscriber`'s `try_init()` function to install the
/// global subscriber. This function keeps the existing subscriber if one is
/// already installed. This means to use your own Subscriber, install it before
/// calling any Cushy code and your subscriber will be the one used.
///
/// ### Disabling tracing support
///
/// The `tracing-output` Cargo feature controls whether tracing is enabled. It
/// is included in `default-features`, but can be omitted to disable tracing
/// support.
pub struct PendingApp {
    app: kludgine::app::PendingApp<WindowCommand>,
    cushy: Cushy,
}

impl PendingApp {
    /// Returns a new app using the provided runtime.
    ///
    /// Unliked `PendingApp::default()`, this function does not initialize
    /// `tracing` support. See
    /// [`with_tracing()`](Self::with_tracing)/[`initialize_tracing()`](Self::initialize_tracing)
    /// to enable Cushy's built-in trace handling.
    pub fn new<Runtime: AppRuntime>(runtime: Runtime) -> Self {
        Self {
            app: kludgine::app::PendingApp::default(),
            cushy: Cushy::new(BoxedRuntime(Box::new(runtime))),
        }
    }

    /// Installs a global `tracing` Subscriber and returns self.
    #[must_use]
    pub fn with_tracing(self) -> Self {
        self.initialize_tracing();
        self
    }

    /// Installs a global `tracing` Subscriber.
    pub fn initialize_tracing(&self) {
        initialize_tracing();
    }

    /// The shared resources this application utilizes.
    #[must_use]
    pub const fn cushy(&self) -> &Cushy {
        &self.cushy
    }

    /// Executes `on_startup` once the application event loop has begun.
    ///
    /// Some APIs are not available until after the application has started
    /// running. For example, `App::monitors` requires the event loop to have
    /// been started.
    pub fn on_startup<F, R>(&mut self, on_startup: F)
    where
        F: FnOnce(&mut App) -> R + Send + 'static,
        R: StartupResult,
    {
        let mut app = self.as_app();
        self.app.on_startup(move |_app| {
            // Accessing some information from this closure needs to use `_app`
            // instead of `App`. For example, accessing monitor information
            // requires the window thread to respond to a message. Trying to do
            // that in this closure would cause the thread to block. So, we
            // execute our on_startup callbacks in their own thread.
            thread::spawn(move || {
                let cushy = app.cushy.clone();
                let _guard = cushy.enter_runtime();
                if let Err(err) = on_startup(&mut app).into_result() {
                    eprintln!("error in on_startup: {err}");
                    exit(-1);
                }
            });
        });
    }
}

impl Run for PendingApp {
    fn run(self) -> crate::Result {
        let _guard = self.cushy.enter_runtime();
        animation::spawn(self.cushy.clone());
        self.app.run()
    }
}

impl Default for PendingApp {
    fn default() -> Self {
        Self::new(DefaultRuntime::default()).with_tracing()
    }
}

impl AsApplication<AppEvent<WindowCommand>> for PendingApp {
    fn as_application(&self) -> &dyn kludgine::app::Application<AppEvent<WindowCommand>> {
        self.app.as_application()
    }

    fn as_application_mut(&mut self) -> &mut dyn kludgine::app::Application<AppEvent<WindowCommand>>
    where
        AppEvent<WindowCommand>: kludgine::app::Message,
    {
        self.app.as_application_mut()
    }
}

pub trait StartupResult {
    fn into_result(self) -> cushy::Result;
}

impl StartupResult for () {
    fn into_result(self) -> crate::Result {
        Ok(())
    }
}

impl<E> StartupResult for Result<(), E>
where
    E: Into<EventLoopError>,
{
    fn into_result(self) -> crate::Result {
        self.map_err(Into::into)
    }
}

/// A runtime associated with the Cushy application.
///
/// This trait is how Cushy adds optional support for `tokio`.
pub trait AppRuntime: Send + Clone + 'static {
    /// The guard type returned from entering the context of the app's runtime.
    type Guard<'a>;

    /// Enter the application's rutime context.
    fn enter(&self) -> Self::Guard<'_>;
}

/// A default application runtime.
///
/// When the `tokio` feature is enabled, a tokio runtime is spawned when this
/// runtime is used in Cushy.
#[derive(Debug, Clone, Default)]
pub struct DefaultRuntime {
    #[cfg(feature = "tokio")]
    tokio: TokioRuntime,
    _private: (),
}

impl AppRuntime for DefaultRuntime {
    type Guard<'a> = DefaultRuntimeGuard<'a>;

    fn enter(&self) -> Self::Guard<'_> {
        DefaultRuntimeGuard {
            #[cfg(feature = "tokio")]
            _tokio: self.tokio.enter(),
            _phantom: PhantomData,
        }
    }
}

pub struct DefaultRuntimeGuard<'a> {
    #[cfg(feature = "tokio")]
    _tokio: ::tokio::runtime::EnterGuard<'a>,
    _phantom: PhantomData<&'a ()>,
}

#[cfg(feature = "tokio")]
mod tokio {
    use std::future::Future;
    use std::ops::Deref;
    use std::task::Poll;
    use std::thread;

    use tokio::runtime::{self, Handle};

    use super::AppRuntime;
    use crate::Lazy;

    /// A spawned `tokio` runtime.
    #[derive(Debug, Clone)]
    pub struct TokioRuntime {
        pub(crate) handle: Handle,
    }

    impl From<Handle> for TokioRuntime {
        fn from(handle: Handle) -> Self {
            Self { handle }
        }
    }

    static TOKIO: Lazy<Handle> = Lazy::new(|| {
        #[cfg(feature = "tokio-multi-thread")]
        let mut rt = runtime::Builder::new_multi_thread();
        #[cfg(not(feature = "tokio-multi-thread"))]
        let mut rt = runtime::Builder::new_current_thread();
        let runtime = rt
            .enable_all()
            .build()
            .expect("failure to initialize tokio");
        let handle = runtime.handle().clone();
        thread::Builder::new()
            .name(String::from("tokio"))
            .spawn(move || {
                runtime.block_on(BlockForever);
            })
            .expect("error spawning tokio thread");
        handle
    });

    impl Default for TokioRuntime {
        fn default() -> Self {
            Self {
                handle: TOKIO.clone(),
            }
        }
    }

    impl Deref for TokioRuntime {
        type Target = Handle;

        fn deref(&self) -> &Self::Target {
            &self.handle
        }
    }

    struct BlockForever;
    impl Future for BlockForever {
        type Output = ();

        fn poll(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Self::Output> {
            Poll::<()>::Pending
        }
    }

    impl AppRuntime for TokioRuntime {
        type Guard<'a> = tokio::runtime::EnterGuard<'a>;

        fn enter(&self) -> Self::Guard<'_> {
            self.handle.enter()
        }
    }
}

#[cfg(feature = "tokio")]
pub use tokio::TokioRuntime;

struct BoxedRuntime(Box<dyn BoxableRuntime>);

impl Clone for BoxedRuntime {
    fn clone(&self) -> Self {
        self.0.cloned()
    }
}

trait BoxableRuntime: Send {
    fn enter_runtime(&self) -> RuntimeGuard<'_>;
    fn cloned(&self) -> BoxedRuntime;
}

impl<T> BoxableRuntime for T
where
    T: AppRuntime,
    for<'a> T::Guard<'a>: BoxableGuard<'a>,
{
    fn enter_runtime(&self) -> RuntimeGuard<'_> {
        RuntimeGuard(Box::new(AppRuntime::enter(self)))
    }

    fn cloned(&self) -> BoxedRuntime {
        BoxedRuntime(Box::new(self.clone()))
    }
}

#[allow(dead_code)]
pub struct RuntimeGuard<'a>(Box<dyn BoxableGuard<'a> + 'a>);

trait BoxableGuard<'a> {}
impl<'a, T> BoxableGuard<'a> for T {}

struct AppSettings {
    multi_click_threshold: Duration,
}

/// Shared resources for a GUI application.
#[derive(Clone)]
pub struct Cushy {
    pub(crate) clipboard: Option<Arc<Mutex<Clipboard>>>,
    pub(crate) fonts: FontCollection,
    settings: Arc<Mutex<AppSettings>>,
    runtime: BoxedRuntime,
}

impl Cushy {
    fn new(runtime: BoxedRuntime) -> Self {
        Self {
            clipboard: Clipboard::new()
                .ok()
                .map(|clipboard| Arc::new(Mutex::new(clipboard))),
            fonts: FontCollection::default(),
            settings: Arc::new(Mutex::new(AppSettings {
                multi_click_threshold: Duration::from_millis(500),
            })),
            runtime,
        }
    }

    /// Returns the duration between two mouse clicks that should be allowed to
    /// elapse for the clicks to be considered separate actions.
    #[must_use]
    pub fn multi_click_threshold(&self) -> Duration {
        self.settings.lock().multi_click_threshold
    }

    /// Sets the maximum time between sequential clicks that should be
    /// considered the same action.
    pub fn set_multi_click_threshold(&self, threshold: Duration) {
        self.settings.lock().multi_click_threshold = threshold;
    }

    /// Returns a locked mutex guard to the OS's clipboard, if one was able to be
    /// initialized when the window opened.
    #[must_use]
    pub fn clipboard_guard(&self) -> Option<MutexGuard<'_, Clipboard>> {
        self.clipboard.as_ref().map(|mutex| mutex.lock())
    }

    /// Returns the font collection that will be loaded in all Cushy windows.
    #[must_use]
    pub fn fonts(&self) -> &FontCollection {
        &self.fonts
    }

    /// Enters the application's runtime context.
    ///
    /// When the `tokio` feature is enabled, the guard returned by this function
    /// allows for functions like `tokio::spawn` to work for the current thread.
    /// Outside of application startup, this function shouldn't need to be
    /// called unless you are manually spawning threads.
    #[must_use]
    pub fn enter_runtime(&self) -> RuntimeGuard<'_> {
        self.runtime.0.enter_runtime()
    }
}

impl Default for Cushy {
    fn default() -> Self {
        Self::new(BoxedRuntime(Box::<DefaultRuntime>::default()))
    }
}

/// A type that is a Cushy application.
pub trait Application: AsApplication<AppEvent<WindowCommand>> {
    /// Returns the shared resources for the application.
    fn cushy(&self) -> &Cushy;
    /// Returns this type as an [`App`] handle.
    fn as_app(&self) -> App;
}

impl Application for PendingApp {
    fn cushy(&self) -> &Cushy {
        &self.cushy
    }

    fn as_app(&self) -> App {
        App {
            app: Some(self.app.as_app()),
            cushy: self.cushy.clone(),
        }
    }
}

/// A handle to a Cushy application.
#[derive(Clone)]
pub struct App {
    app: Option<kludgine::app::App<WindowCommand>>,
    cushy: Cushy,
}

impl App {
    pub(crate) fn standalone() -> Self {
        Self {
            app: None,
            cushy: Cushy::default(),
        }
    }

    /// Returns a snapshot of information about the monitors connected to this
    /// device.
    ///
    /// Returns None if the app is not currently running.
    #[must_use]
    pub fn monitors(&self) -> Option<Monitors> {
        self.app.as_ref().and_then(kludgine::app::App::monitors)
    }

    /// Creates a guard that prevents this app from shutting down.
    ///
    /// If the app is not currently running, this function returns None.
    ///
    /// Once a guard is allocated the app will not be closed automatically when
    /// the final window is closed. If the final shutdown guard is dropped while
    /// no windows are open, the app will be closed.
    #[allow(clippy::missing_panics_doc, clippy::must_use_candidate)]
    pub fn prevent_shutdown(&self) -> Option<ShutdownGuard> {
        self.app
            .as_ref()
            .and_then(kludgine::app::App::prevent_shutdown)
    }

    /// Executes `callback` on the main event loop thread.
    ///
    /// Returns true if the callback was able to be sent to be executed. The app
    /// may still terminate before the callback is executed regardless of the
    /// result of this function. The only way to know with certainty that
    /// `callback` is executed is to have `callback` notify the caller of its
    /// completion.
    pub fn execute<Callback>(&self, callback: Callback) -> bool
    where
        Callback: FnOnce(&ExecutingApp<'_, WindowCommand>) + Send + 'static,
    {
        self.app.as_ref().map_or(false, |app| app.execute(callback))
    }
}

/// A guard preventing an [`App`] from shutting down.
pub type ShutdownGuard = kludgine::app::ShutdownGuard<WindowCommand>;

impl Application for App {
    fn cushy(&self) -> &Cushy {
        &self.cushy
    }

    fn as_app(&self) -> App {
        self.clone()
    }
}

impl AsApplication<AppEvent<WindowCommand>> for App {
    fn as_application(&self) -> &dyn kludgine::app::Application<AppEvent<WindowCommand>> {
        self.app
            .as_ref()
            .map(AsApplication::as_application)
            .expect("no app")
    }

    fn as_application_mut(&mut self) -> &mut dyn kludgine::app::Application<AppEvent<WindowCommand>>
    where
        AppEvent<WindowCommand>: kludgine::app::Message,
    {
        self.app
            .as_mut()
            .map(AsApplication::as_application_mut)
            .expect("no app")
    }
}

/// A type that can be run as an application.
pub trait Run: Sized {
    /// Runs the provided type, returning `Ok(())` upon successful execution and
    /// program exit. Note that this function may not ever return on some
    /// platforms.
    fn run(self) -> crate::Result;
}

/// A type that can be opened as a window in an application.
pub trait Open: Sized {
    /// Opens the provided type as a window inside of `app`.
    fn open<App>(self, app: &mut App) -> crate::Result<WindowHandle>
    where
        App: Application + ?Sized;

    /// Runs the provided type inside of the pending `app`, returning `Ok(())`
    /// upon successful execution and program exit. Note that this function may
    /// not ever return on some platforms.
    fn run_in(self, app: PendingApp) -> crate::Result;
}
