use std::ops::Deref;
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;

use intentional::Assert;
use kludgine::app::winit::event::Modifiers;
use kludgine::app::winit::keyboard::ModifiersState;

/// Invokes the provided macro with a pattern that can be matched using this
/// `macro_rules!` expression: `$($type:ident $field:tt $var:ident),+`, where
/// `$type` is an identifier to use for the generic parameter and `$field` is
/// the field index inside of the tuple.
///
/// If `impl_all_tuples!(macro_name, 2)` is provided, an additional identifier
/// will be provided before `$field`.
macro_rules! impl_all_tuples {
    ($macro_name:ident) => {
        impl_all_tuples!($macro_name, 1);
    };
    ($macro_name:ident, 1) => {
        $macro_name!(T0 0 t0);
        $macro_name!(T0 0 t0, T1 1 t1);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3, T4 4 t4);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3, T4 4 t4, T5 5 t5);
    };
    ($macro_name:ident, 2) => {
        $macro_name!(T0 Y0 0 t0);
        $macro_name!(T0 Y0 0 t0, T1 Y1 1 t1);
        $macro_name!(T0 Y0 0 t0, T1 Y1 1 t1, T2 Y2 2 t2);
        $macro_name!(T0 Y0 0 t0, T1 Y1 1 t1, T2 Y2 2 t2, T3 Y3 3 t3);
        $macro_name!(T0 Y0 0 t0, T1 Y1 1 t1, T2 Y2 2 t2, T3 Y3 3 t3, T4 Y4 4 t4);
        $macro_name!(T0 Y0 0 t0, T1 Y1 1 t1, T2 Y2 2 t2, T3 Y3 3 t3, T4 Y4 4 t4, T5 Y5 5 t5);
    };
}

/// Invokes a function with a clone of `self`.
pub trait WithClone: Sized {
    /// The type that results from cloning.
    type Cloned;

    /// Maps `with` with the results of cloning `self`.
    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R;
}

macro_rules! impl_with_clone {
    ($($name:ident $field:tt $var:ident),+) => {
        impl<'a, $($name: Clone,)+> WithClone for ($(&'a $name,)+)
        {
            type Cloned = ($($name,)+);

            fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
                with(($(self.$field.clone(),)+))
            }
        }
    };
}

impl<'a, T> WithClone for &'a T
where
    T: Clone,
{
    type Cloned = T;

    fn with_clone<R>(&self, with: impl FnOnce(Self::Cloned) -> R) -> R {
        with((*self).clone())
    }
}

impl_all_tuples!(impl_with_clone);

/// Helper functions for [`Modifiers`] and [`ModifiersState`].
pub trait ModifiersExt {
    /// Returns true if the current state includes the platform's primary
    /// shortcut key.
    ///
    /// For Apple based platforms, this returns true if a "super" modifier is
    /// pressed. This corresponds to the Apple/Command key.
    ///
    /// For all other platforms, this returns true if a control key is pressed.
    fn primary(&self) -> bool;

    /// Returns true if only the [primary](Self::primary()) modifier key is
    /// pressed.
    fn only_primary(&self) -> bool;

    /// Returns true if only a shift modifier key is pressed.
    fn only_shift(&self) -> bool;
    /// Returns true if only a control modifier key is pressed.
    fn only_control(&self) -> bool;
    /// Returns true if only an alt modifier key is pressed.
    fn only_alt(&self) -> bool;
    /// Returns true if only a super modifier key is pressed.
    fn only_super(&self) -> bool;

    /// Returns true if the platform-specific modifier for word-selection is
    /// pressed.
    ///
    /// For Apple-based platforms, this returns true if an "alt" key is pressed.
    /// This corresponds to the Option key.
    ///
    /// For all other platforms, this returns true if a control key is pressed.
    fn word_select(&self) -> bool;

    /// Returns true if the current modifier state might be a shortcut key.
    ///
    /// This returns true if either the control key, alt key, or super key are
    /// pressed.
    fn possible_shortcut(&self) -> bool;
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

    fn possible_shortcut(&self) -> bool {
        self.control_key() || self.alt_key() || self.super_key()
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn only_primary(&self) -> bool {
        self.super_key() && !self.shift_key() && !self.control_key() && !self.alt_key()
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    fn only_primary(&self) -> bool {
        self.control_key() && !self.shift_key() && !self.super_key() && !self.alt_key()
    }

    fn only_shift(&self) -> bool {
        self.shift_key() && !self.control_key() && !self.super_key() && !self.alt_key()
    }

    fn only_control(&self) -> bool {
        self.control_key() && !self.shift_key() && !self.super_key() && !self.alt_key()
    }

    fn only_alt(&self) -> bool {
        self.alt_key() && !self.control_key() && !self.shift_key() && !self.super_key()
    }

    fn only_super(&self) -> bool {
        self.super_key() && !self.control_key() && !self.shift_key() && !self.alt_key()
    }
}

impl ModifiersExt for Modifiers {
    fn primary(&self) -> bool {
        self.state().primary()
    }

    fn word_select(&self) -> bool {
        self.state().word_select()
    }

    fn possible_shortcut(&self) -> bool {
        self.state().word_select()
    }

    fn only_primary(&self) -> bool {
        self.state().only_primary()
    }

    fn only_shift(&self) -> bool {
        self.state().only_shift()
    }

    fn only_control(&self) -> bool {
        self.state().only_control()
    }

    fn only_alt(&self) -> bool {
        self.state().only_alt()
    }

    fn only_super(&self) -> bool {
        self.state().only_super()
    }
}

/// A [`OnceLock`]-based lazy initializer.
pub struct Lazy<T> {
    init: fn() -> T,
    once: OnceLock<T>,
}

impl<T> Lazy<T> {
    /// Returns a type that initializes itself once upon being accessed.
    ///
    /// `init` is guaranteed to be called only once, but this type can't accept
    /// `FnOnce` generic types due to being unable to allocate a `Box<dyn T>` in
    /// `const` or being able to give a name to the type of a function so that
    /// users could use this type in static variables.
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

pub trait BgFunction: FnOnce() + Send + 'static {}

pub fn run_in_bg<F>(f: F)
where
    F: BgFunction,
{
    static BG_THREAD: Lazy<SyncSender<Box<dyn BgFunction>>> = Lazy::new(|| {
        let (sender, receiver) = mpsc::sync_channel::<Box<dyn BgFunction>>(16);
        std::thread::Builder::new()
            .name(String::from("background"))
            .spawn(move || {
                while let Ok(callback) = receiver.recv() {
                    (callback)();
                }
            })
            .assert("error spawning bg thread");
        sender
    });

    BG_THREAD
        .send(Box::new(f))
        .assert("background thread not running");
}

impl<T> BgFunction for T where T: FnOnce() + Send + 'static {}
