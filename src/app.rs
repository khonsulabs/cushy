use std::sync::{Arc, Mutex, MutexGuard};

use arboard::Clipboard;

use crate::utils::IgnorePoison;

/// A GUI application.
#[derive(Clone)]
pub struct Gooey {
    pub(crate) clipboard: Option<Arc<Mutex<Clipboard>>>,
}

impl Default for Gooey {
    fn default() -> Self {
        Self {
            clipboard: Clipboard::new()
                .ok()
                .map(|clipboard| Arc::new(Mutex::new(clipboard))),
        }
    }
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
