use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kludgine::app::winit::event::KeyEvent;
use kludgine::app::winit::keyboard::Key;

use crate::widget::{EventHandling, HANDLED, UNHANDLED};

#[derive(Clone, Debug)]
#[must_use]
pub struct Tick {
    data: Arc<TickData>,
    handled_keys: HashSet<Key>,
}

impl Tick {
    pub fn rendered(&self) {
        self.data.rendered_frame.fetch_add(1, Ordering::AcqRel);

        self.data.sync.notify_one();
    }

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
            UNHANDLED
        }
    }
}

#[derive(Default, Debug)]
pub struct WatchedInput {
    pub keys: HashSet<Key>,
}

#[derive(Debug)]
struct TickData {
    state: Mutex<TickState>,
    period: Duration,
    sync: Condvar,
    rendered_frame: AtomicUsize,
}

impl TickData {
    fn state(&self) -> MutexGuard<'_, TickState> {
        self.state
            .lock()
            .map_or_else(PoisonError::into_inner, |g| g)
    }
}

#[derive(Debug)]
struct TickState {
    last_time: Instant,
    next_target: Instant,
    keep_running: bool,
    frame: usize,
    input: WatchedInput,
}

impl Tick {
    pub fn new<F>(tick_every: Duration, tick: F) -> Self
    where
        F: FnMut(Duration, &WatchedInput) + Send + 'static,
    {
        let now = Instant::now();
        let data = Arc::new(TickData {
            state: Mutex::new(TickState {
                last_time: now,
                next_target: now,
                keep_running: true,
                frame: 0,
                input: WatchedInput::default(),
            }),
            period: tick_every,
            sync: Condvar::new(),
            rendered_frame: AtomicUsize::new(0),
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

    pub fn fps<F>(frames_per_second: u32, tick: F) -> Self
    where
        F: FnMut(Duration, &WatchedInput) + Send + 'static,
    {
        Self::new(Duration::from_secs(1) / frames_per_second, tick)
    }

    pub fn handled_keys(mut self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.handled_keys.extend(keys);
        self
    }
}

fn tick_loop<F>(data: &TickData, mut tick: F)
where
    F: FnMut(Duration, &WatchedInput),
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
        // TODO we need a way to batch updates for a context so that during a
        // tick, no changed values trigger a redraw until we are done with the
        // tick. Otherwise, a frame may start being rendered while we're still
        // evaluating the tick since it's in its own thread.
        tick(elapsed, &state.input);
        state.next_target = (state.next_target + data.period).max(now);
        state.last_time = now;

        // Wait for a frame to be rendered.
        while state.keep_running {
            let current_frame = data.rendered_frame.load(Ordering::Acquire);
            if state.frame == current_frame {
                state = data
                    .sync
                    .wait(state)
                    .map_or_else(PoisonError::into_inner, |g| g);
            } else {
                break;
            }
        }
    }
}
