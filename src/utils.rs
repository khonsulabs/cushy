use std::ops::Deref;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Condvar, OnceLock, PoisonError};

use intentional::Assert;
use kludgine::app::winit::event::Modifiers;
use kludgine::app::winit::keyboard::ModifiersState;

/// This [`Condvar`] is a wrapper that on Mac OS/iOS asserts unwind safety. On
/// all other platforms, this is a transparent wrapper around `Condvar`. See
/// <https://github.com/rust-lang/rust/issues/118009> for more information.
#[derive(Debug, Default)]
pub struct UnwindsafeCondvar(
    #[cfg(any(target_os = "ios", target_os = "macos"))] std::panic::AssertUnwindSafe<Condvar>,
    #[cfg(not(any(target_os = "ios", target_os = "macos")))] Condvar,
);

impl Deref for UnwindsafeCondvar {
    type Target = Condvar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UnwindsafeCondvar {
    pub const fn new() -> Self {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            Self(AssertUnwindSafe(Condvar::new()))
        }

        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            Self(Condvar::new())
        }
    }
}

/// Invokes the provided macro with a pattern that can be matched using this
/// `macro_rules!` expression: `$($type:ident $field:tt $var:ident),+`, where `$type` is an
/// identifier to use for the generic parameter and `$field` is the field index
/// inside of the tuple.
macro_rules! impl_all_tuples {
    ($macro_name:ident) => {
        $macro_name!(T0 0 t0);
        $macro_name!(T0 0 t0, T1 1 t1);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3, T4 4 t4);
        $macro_name!(T0 0 t0, T1 1 t1, T2 2 t2, T3 3 t3, T4 4 t4, T5 5 t5);
    }
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

pub trait ModifiersExt {
    fn primary(&self) -> bool;
    fn word_select(&self) -> bool;

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

pub trait IgnorePoison {
    type Unwrapped;
    fn ignore_poison(self) -> Self::Unwrapped;
}

impl<T> IgnorePoison for Result<T, PoisonError<T>> {
    type Unwrapped = T;

    fn ignore_poison(self) -> Self::Unwrapped {
        self.map_or_else(PoisonError::into_inner, |g| g)
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
