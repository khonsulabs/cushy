use std::ops::Deref;
use std::sync::OnceLock;

use kludgine::app::winit::event::Modifiers;
use kludgine::app::winit::keyboard::ModifiersState;

pub trait ModifiersExt {
    fn primary(&self) -> bool;
    fn word_select(&self) -> bool;
}

impl ModifiersExt for ModifiersState {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn primary(&self) -> bool {
        self.super_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn primary(&self) -> bool {
        self.control_key()
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn word_select(&self) -> bool {
        self.alt_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn word_select(&self) -> bool {
        self.control_key()
    }
}

impl ModifiersExt for Modifiers {
    fn primary(&self) -> bool {
        self.state().primary()
    }

    fn word_select(&self) -> bool {
        self.state().word_select()
    }
}

pub struct Lazy<T> {
    init: fn() -> T,
    once: OnceLock<T>,
}

impl<T> Lazy<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            init,
            once: OnceLock::new(),
        }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.once.get_or_init(self.init)
    }
}
