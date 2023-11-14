use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use kludgine::app::winit::event::KeyEvent;
use kludgine::app::winit::keyboard::Key;

use crate::context::WidgetContext;
use crate::utils::IgnorePoison;
use crate::value::Dynamic;
use crate::widget::{EventHandling, HANDLED, IGNORED};

/// A fixed-rate callback that provides access to tracked input on its
/// associated widget.
#[derive(Clone, Debug)]
#[must_use]
pub struct Tick {
    data: Arc<TickData>,
    handled_keys: HashSet<Key>,
}

impl Tick {
    /// Signals that this widget has been redrawn.
    pub fn rendered(&self, context: &WidgetContext<'_, '_>) {
        context.redraw_when_changed(&self.data.tick_number);

        self.data.sync.notify_one();
    }

    /// Processes `input`.
    ///
    /// If the event matches a key that has been marked as handled, [`HANDLED`]
    /// will be returned. Otherwise, [`IGNORED`] will be returned,
    #[must_use]
    pub fn key_input(&self, input: &KeyEvent) -> EventHandling {
        let mut state = self.data.state();
        if input.state.is_pressed() {
            state.input.keys.insert(input.logical_key.clone());
        } else {
            state.input.keys.remove(&input.logical_key);
        }
        drop(state);

        if self.handled_keys.contains(&input.logical_key) {
            HANDLED
        } else {
            IGNORED
        }
    }

    /// Returns a new tick that invokes `tick`, aiming to repeat at the given
    /// duration.
    pub fn new<F>(tick_every: Duration, tick: F) -> Self
    where
        F: FnMut(Duration, &InputState) + Send + 'static,
    {
        let now = Instant::now();
        let data = Arc::new(TickData {
            state: Mutex::new(TickState {
                last_time: now,
                next_target: now,
                keep_running: true,
                frame: 0,
                input: InputState::default(),
            }),
            period: tick_every,
            sync: Condvar::new(),
            rendered_frame: AtomicUsize::new(0),
            tick_number: Dynamic::default(),
        });

        std::thread::spawn({
            let data = data.clone();
            move || tick_loop(&data, tick)
        });

        Self {
            data,
            handled_keys: HashSet::new(),
        }
    }

    /// Returns a new tick that invokes `tick` at a target number of times per
    /// second.
    pub fn times_per_second<F>(times_per_second: u32, tick: F) -> Self
    where
        F: FnMut(Duration, &InputState) + Send + 'static,
    {
        Self::new(Duration::from_secs(1) / times_per_second, tick)
    }

    /// Returns a new tick that redraws its associated widget at a target rate
    /// of `x times_per_second`.
    pub fn redraws_per_second(times_per_second: u32) -> Self {
        Self::times_per_second(times_per_second, |_, _| {})
    }

    /// Adds the collection of [`Key`]s to the list that are handled, and
    /// returns self.
    ///
    /// The list of keys provided will be prevented from propagating.
    pub fn handled_keys(mut self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.handled_keys.extend(keys);
        self
    }
}

/// The current state of input during the execution of a [`Tick`].
#[derive(Default, Debug)]
pub struct InputState {
    /// A collection of all keys currently pressed.
    pub keys: HashSet<Key>,
}

#[derive(Debug)]
struct TickData {
    state: Mutex<TickState>,
    period: Duration,
    sync: Condvar,
    rendered_frame: AtomicUsize,
    tick_number: Dynamic<u64>,
}

impl TickData {
    fn state(&self) -> MutexGuard<'_, TickState> {
        self.state.lock().ignore_poison()
    }
}

#[derive(Debug)]
struct TickState {
    last_time: Instant,
    next_target: Instant,
    keep_running: bool,
    frame: usize,
    input: InputState,
}

fn tick_loop<F>(data: &TickData, mut tick: F)
where
    F: FnMut(Duration, &InputState),
{
    let mut state = data.state();
    while state.keep_running {
        let mut now = Instant::now();
        match state.next_target.checked_duration_since(now) {
            Some(remaining) if remaining > Duration::ZERO => {
                drop(state);
                std::thread::sleep(remaining);
                state = data.state();

                now = Instant::now();
            }
            _ => {}
        }

        let elapsed = now
            .checked_duration_since(state.last_time)
            .expect("instant never decreases");
        state.frame += 1;

        tick(elapsed, &state.input);
        state.next_target = (state.next_target + data.period).max(now);
        state.last_time = now;

        // Signal that we have a new frame, which will cause the widget to
        // redraw.
        data.tick_number.map_mut(|tick| *tick += 1);

        // Wait for a frame to be rendered.
        while state.keep_running {
            let current_frame = data.rendered_frame.load(Ordering::Acquire);
            if state.frame == current_frame {
                state = data.sync.wait(state).ignore_poison();
            } else {
                break;
            }
        }
    }
}
