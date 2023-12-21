use std::sync::{Arc, Mutex, MutexGuard};

use arboard::Clipboard;
use kludgine::app::{AppEvent, AsApplication};

use crate::utils::IgnorePoison;
use crate::window::sealed::WindowCommand;

/// A Gooey application that has not started running yet.
pub struct PendingApp {
    app: kludgine::app::PendingApp<WindowCommand>,
    gooey: Gooey,
}

impl PendingApp {
    /// The shared resources this application utilizes.
    pub const fn gooey(&self) -> &Gooey {
        &self.gooey
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
            gooey: Gooey {
                clipboard: Clipboard::new()
                    .ok()
                    .map(|clipboard| Arc::new(Mutex::new(clipboard))),
            },
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
pub struct Gooey {
    pub(crate) clipboard: Option<Arc<Mutex<Clipboard>>>,
}

impl Gooey {
    /// Returns a locked mutex guard to the OS's clipboard, if one was able to be
    /// initialized when the window opened.
    #[must_use]
    pub fn clipboard_guard(&self) -> Option<MutexGuard<'_, Clipboard>> {
        self.clipboard
            .as_ref()
            .map(|mutex| mutex.lock().ignore_poison())
    }
}

/// A type that is a Gooey application.
pub trait Application: AsApplication<AppEvent<WindowCommand>> {
    /// Returns the shared resources for the application.
    fn gooey(&self) -> &Gooey;
    /// Returns this type as an [`App`] handle.
    fn as_app(&self) -> App;
}

impl Application for PendingApp {
    fn gooey(&self) -> &Gooey {
        &self.gooey
    }

    fn as_app(&self) -> App {
        App {
            app: self.app.as_app(),
            gooey: self.gooey.clone(),
        }
    }
}

/// A handle to a Gooey application.
#[derive(Clone)]
pub struct App {
    app: kludgine::app::App<WindowCommand>,
    gooey: Gooey,
}

impl Application for App {
    fn gooey(&self) -> &Gooey {
        &self.gooey
    }

    fn as_app(&self) -> App {
        self.clone()
    }
}

impl AsApplication<AppEvent<WindowCommand>> for App {
    fn as_application(&self) -> &dyn kludgine::app::Application<AppEvent<WindowCommand>> {
        self.app.as_application()
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
    fn open<App>(self, app: &App) -> crate::Result
    where
        App: Application;

    /// Runs the provided type inside of the pending `app`, returning `Ok(())`
    /// upon successful execution and program exit. Note that this function may
    /// not ever return on some platforms.
    fn run_in(self, app: PendingApp) -> crate::Result;
}
