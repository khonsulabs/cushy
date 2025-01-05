use std::time::{Duration, Instant};

use cushy::value::{Destination, Dynamic, DynamicReader, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::progress::Progressable;
use cushy::widgets::slider::Slidable;
use cushy::{Open, PendingApp};
use figures::units::Lp;

#[derive(PartialEq, Debug, Clone)]
struct Timer {
    started_at: Instant,
    duration: Duration,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            started_at: Instant::now() - Duration::from_secs(1),
            duration: Duration::from_secs(1),
        }
    }
}

fn main() -> cushy::Result {
    let pending = PendingApp::default();
    let cushy = pending.cushy().clone();
    let _runtime = cushy.enter_runtime();

    let timer = Dynamic::<Timer>::default();
    let duration = timer.linked_accessor(|timer| &timer.duration, |timer| &mut timer.duration);

    let elapsed = spawn_timer(&timer);
    let duration_label = duration.map_each(|duration| format!("{}s", duration.as_secs_f32()));

    elapsed
        .progress_bar_between(duration.map_each_cloned(|duration| Duration::ZERO..=duration))
        .fit_horizontally()
        .and(duration_label)
        .and(
            "Duration"
                .and(
                    duration
                        .slider_between(Duration::ZERO, Duration::from_secs(30))
                        .expand_horizontally(),
                )
                .into_columns(),
        )
        .and("Reset".into_button().on_click(move |_| {
            timer.lock().started_at = Instant::now();
        }))
        .into_rows()
        .pad()
        .width(Lp::inches(4))
        .into_window()
        .titled("Timer")
        .resize_to_fit(true)
        .run_in(pending)
}

fn spawn_timer(timer: &Dynamic<Timer>) -> DynamicReader<Duration> {
    let timer = timer.create_reader();
    let elapsed = Dynamic::new(timer.map_ref(|timer| timer.duration));
    let elapsed_reader = elapsed.create_reader();
    std::thread::spawn(move || loop {
        let settings = timer.get();

        // Update the elapsed time, clamping to the duration of the timer.
        let duration_since_started = settings.started_at.elapsed().min(settings.duration);
        elapsed.set(duration_since_started);

        if duration_since_started < settings.duration {
            // The timer is still running, "tick" the timer by sleeping and
            // allow the loop to continue.
            std::thread::sleep(Duration::from_millis(16));
        } else {
            // Block the thread until the timer settings have been changed.
            timer.block_until_updated();
        }
    });
    elapsed_reader
}
