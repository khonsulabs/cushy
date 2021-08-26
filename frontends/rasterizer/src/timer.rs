use std::time::{Duration, Instant};

use flume::RecvTimeoutError;
use gooey_core::{Callback, NativeTimer, Timer, WeakTimer};
use once_cell::sync::OnceCell;

static TIMER_THREAD_SENDER: OnceCell<flume::Sender<Command>> = OnceCell::new();

#[derive(Debug)]
pub struct ThreadTimer {
    period: Duration,
    repeating: bool,
    callback: Callback,
}

#[derive(Debug)]
struct Data {}

impl ThreadTimer {
    pub fn schedule(callback: Callback, period: Duration, repeating: bool) -> Timer {
        let next_time = Instant::now() + period;
        let thread_timer = Self {
            period,
            repeating,
            callback,
        };
        let timer = Timer::from_native(thread_timer);
        TIMER_THREAD_SENDER
            .get_or_init(|| {
                let (sender, receiver) = flume::unbounded();
                std::thread::Builder::new()
                    .name(String::from("gooey-timers"))
                    .spawn(move || timer_thread(receiver))
                    .unwrap();
                sender
            })
            .send(Command::Schedule(ScheduledTimer {
                next_time,
                timer: timer.downgrade(),
            }))
            .unwrap();
        timer
    }
}

impl NativeTimer for ThreadTimer {}

#[derive(Debug)]
enum Command {
    Schedule(ScheduledTimer),
}

fn timer_thread(commands: flume::Receiver<Command>) {
    // Ordered list of timers, sorted by the scheduled instant.
    let mut timers = Vec::<ScheduledTimer>::default();

    while let Ok(event) = next_event(&commands, &mut timers) {
        match event {
            Event::TimerElapsed => {
                let now = Instant::now();
                while timers.first().map_or(false, |timer| timer.next_time <= now) {
                    let mut elapsed_timer = timers.remove(0);
                    if let Some(timer) = elapsed_timer.timer.upgrade() {
                        let thread_timer = timer.native::<ThreadTimer>().unwrap();
                        thread_timer.callback.invoke(());

                        if thread_timer.repeating {
                            elapsed_timer.next_time = now.checked_add(thread_timer.period).unwrap();
                            schedule_timer(elapsed_timer, &mut timers);
                        }
                    }
                }
            }
            Event::Command(Command::Schedule(timer)) => {
                schedule_timer(timer, &mut timers);
            }
        }
    }
}

fn schedule_timer(timer: ScheduledTimer, timers: &mut Vec<ScheduledTimer>) {
    let insert_at = timers
        .binary_search_by(|a| a.next_time.cmp(&timer.next_time))
        .map_or_else(|e| e, |i| i);
    timers.insert(insert_at, timer);
}

#[derive(Debug)]
struct ScheduledTimer {
    next_time: Instant,
    timer: WeakTimer,
}

#[derive(Debug)]
enum Event {
    TimerElapsed,
    Command(Command),
}

fn next_event(
    commands: &flume::Receiver<Command>,
    timers: &mut Vec<ScheduledTimer>,
) -> Result<Event, flume::RecvError> {
    match duration_until_next_timer(timers) {
        Ok(Some(remaining_time)) => match commands.recv_timeout(remaining_time) {
            Ok(command) => Ok(Event::Command(command)),
            Err(RecvTimeoutError::Timeout) => Ok(Event::TimerElapsed),
            Err(RecvTimeoutError::Disconnected) => Err(flume::RecvError::Disconnected),
        },
        Ok(None) => commands.recv().map(Event::Command),
        Err(_) => Ok(Event::TimerElapsed),
    }
}

/// Returns Err(()) when a timer is already meant to fire. This should
/// practically only happen when a timer *almost* fired but didn't quite fire,
/// but the amount of elapsed time on this thread is such that when we're ready
/// to try to sleep the thread, the next timer was ready to fire already. A
/// result of `Ok(None)` means that there are no timers waiting.
fn duration_until_next_timer(timers: &mut Vec<ScheduledTimer>) -> Result<Option<Duration>, ()> {
    let now = Instant::now();

    timers.first().map_or(Ok(None), |timer| {
        match timer.next_time.checked_duration_since(now) {
            Some(remaining) => Ok(Some(remaining)),
            None => Err(()),
        }
    })
}
