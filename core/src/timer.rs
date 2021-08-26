use std::{
    fmt::Debug,
    sync::{Arc, Weak},
    time::Duration,
};

use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use crate::{AnyFrontend, AnySendSync, Callback};

/// Invokes a [`Callback`] after a delay.
#[derive(Debug, Clone)]
#[must_use]
pub struct Timer {
    native: Arc<Mutex<dyn NativeTimer>>,
}

/// A weak reference to a [`Timer`]. Uses [`Arc`] and [`Weak`] under the hood.
#[derive(Debug, Clone)]
#[must_use]
pub struct WeakTimer(Weak<Mutex<dyn NativeTimer>>);

impl WeakTimer {
    /// Attempts to retrieve the [`Timer`], if at least one [`Timer`] instance
    /// still remains alive. Returns None if all references to the original
    /// timer were dropped.
    #[must_use]
    pub fn upgrade(&self) -> Option<Timer> {
        self.0.upgrade().map(|native| Timer { native })
    }
}

impl Timer {
    /// Returns a new instance from a [`NativeTimer`] implementor.
    pub fn from_native<N: NativeTimer>(native: N) -> Self {
        Self {
            native: Arc::new(Mutex::new(native)),
        }
    }

    /// Returns the underlying [`NativeTimer`] implementor, through a
    /// [`MappedMutexGuard`]. Holding onto this guard will prevent any other
    /// threads from interacting with this timer.
    #[must_use]
    pub fn native<N: NativeTimer>(&self) -> Option<MappedMutexGuard<'_, N>> {
        let guard = self.native.lock();
        MutexGuard::try_map(guard, |native| native.as_mut_any().downcast_mut()).ok()
    }

    /// Returns a weak reference to this timer. Holding onto a [`WeakTimer`]
    /// does not prevent a timer from being unscheduled if all [`Timer`]s are
    /// dropped.
    pub fn downgrade(&self) -> WeakTimer {
        WeakTimer(Arc::downgrade(&self.native))
    }
}

/// A native timer implementation.
pub trait NativeTimer: AnySendSync {}

/// A [`Timer`] that hasn't been scheduled.
#[must_use]
#[derive(Debug, Clone)]
pub struct UnscheduledTimer<'a> {
    frontend: &'a dyn AnyFrontend,
    callback: Callback,
    period: Duration,
    repeating: bool,
}

impl<'a> UnscheduledTimer<'a> {
    pub(crate) fn new(period: Duration, callback: Callback, frontend: &'a dyn AnyFrontend) -> Self {
        Self {
            frontend,
            callback,
            period,
            repeating: false,
        }
    }

    /// Enables repeating this timer for each period.
    pub fn repeating(mut self) -> Self {
        self.repeating = true;
        self
    }

    /// Schedules the timer.
    pub fn schedule(self) -> Timer {
        self.frontend
            .schedule_timer(self.callback, self.period, self.repeating)
    }
}
