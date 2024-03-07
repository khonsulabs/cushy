use std::sync::{Arc, Mutex, MutexGuard};

use arboard::Clipboard;
use kludgine::app::{AppEvent, AsApplication};

use crate::fonts::FontCollection;
use crate::utils::IgnorePoison;
use crate::window::sealed::WindowCommand;
use crate::window::WindowHandle;

/// A Cushy application that has not started running yet.
pub struct PendingApp {
    app: kludgine::app::PendingApp<WindowCommand>,
    cushy: Cushy,
}

impl PendingApp {
    /// The shared resources this application utilizes.
    #[must_use]
    pub const fn cushy(&self) -> &Cushy {
        &self.cushy
    }
}

impl Run for PendingApp {
    fn run(self) -> crate::Result {
        self.app.run()
    }
}

impl Default for PendingApp {
    fn default() -> Self {
        Self {
            app: kludgine::app::PendingApp::default(),
            cushy: Cushy::new(),
        }
    }
}

impl AsApplication<AppEvent<WindowCommand>> for PendingApp {
    fn as_application(&self) -> &dyn kludgine::app::Application<AppEvent<WindowCommand>> {
        self.app.as_application()
    }
}

/// Shared resources for a GUI application.
#[derive(Clone)]
pub struct Cushy {
    pub(crate) clipboard: Option<Arc<Mutex<Clipboard>>>,
    pub(crate) fonts: FontCollection,
}

impl Cushy {
    pub(crate) fn new() -> Self {
        Self {
            clipboard: Clipboard::new()
                .ok()
                .map(|clipboard| Arc::new(Mutex::new(clipboard))),
            fonts: FontCollection::default(),
        }
    }

    /// Returns a locked mutex guard to the OS's clipboard, if one was able to be
    /// initialized when the window opened.
    #[must_use]
    pub fn clipboard_guard(&self) -> Option<MutexGuard<'_, Clipboard>> {
        self.clipboard
            .as_ref()
            .map(|mutex| mutex.lock().ignore_poison())
    }

    /// Returns the font collection that will be loaded in all Cushy windows.
    #[must_use]
    pub fn fonts(&self) -> &FontCollection {
        &self.fonts
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
    fn open<App>(self, app: &App) -> crate::Result<Option<WindowHandle>>
    where
        App: Application + ?Sized;

    /// Runs the provided type inside of the pending `app`, returning `Ok(())`
    /// upon successful execution and program exit. Note that this function may
    /// not ever return on some platforms.
    fn run_in(self, app: PendingApp) -> crate::Result;
}
