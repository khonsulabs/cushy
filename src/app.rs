use std::marker::PhantomData;
use std::sync::Arc;

use arboard::Clipboard;
use kludgine::app::{AppEvent, AsApplication};
use parking_lot::{Mutex, MutexGuard};

use crate::animation;
use crate::fonts::FontCollection;
use crate::window::sealed::WindowCommand;
use crate::window::WindowHandle;

/// A Cushy application that has not started running yet.
pub struct PendingApp {
    app: kludgine::app::PendingApp<WindowCommand>,
    cushy: Cushy,
}

impl PendingApp {
    /// Returns a new app using the provided runtime.
    pub fn new<Runtime: AppRuntime>(runtime: Runtime) -> Self {
        Self {
            app: kludgine::app::PendingApp::default(),
            cushy: Cushy::new(BoxedRuntime(Box::new(runtime))),
        }
    }

    /// The shared resources this application utilizes.
    #[must_use]
    pub const fn cushy(&self) -> &Cushy {
        &self.cushy
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
        Self::new(DefaultRuntime::default())
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

/// Shared resources for a GUI application.
#[derive(Clone)]
pub struct Cushy {
    pub(crate) clipboard: Option<Arc<Mutex<Clipboard>>>,
    pub(crate) fonts: FontCollection,
    runtime: BoxedRuntime,
}

impl Cushy {
    fn new(runtime: BoxedRuntime) -> Self {
        Self {
            clipboard: Clipboard::new()
                .ok()
                .map(|clipboard| Arc::new(Mutex::new(clipboard))),
            fonts: FontCollection::default(),
            runtime,
        }
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
    fn open<App>(self, app: &mut App) -> crate::Result<Option<WindowHandle>>
    where
        App: Application + ?Sized;

    /// Runs the provided type inside of the pending `app`, returning `Ok(())`
    /// upon successful execution and program exit. Note that this function may
    /// not ever return on some platforms.
    fn run_in(self, app: PendingApp) -> crate::Result;
}
