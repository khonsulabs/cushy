use std::time::{Duration, Instant};

use cushy::reactive::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::label::Displayable;
use cushy::Run;

fn main() -> cushy::Result {
    let channel_counter = Dynamic::new(0_usize);
    let channel_counter_label = channel_counter.to_label();
    let sender = cushy::reactive::channel::build()
        .on_receive({
            move |_| {
                std::thread::sleep(Duration::from_secs(1));
                *channel_counter.lock() += 1;
            }
        })
        .finish();

    let dynamic_counter = Dynamic::new(0_usize);
    let dynamic_counter_label = dynamic_counter.to_label();
    let dynamic_value = Dynamic::new(Instant::now());
    // We use a `for_each_subsequent_cloned` to only execute the callback after
    // the current value changes, and the `cloned` version ensures the dynamic
    // isn't locked while the callback is being executed.
    dynamic_value
        .for_each_subsequent_cloned(move |_| {
            std::thread::sleep(Duration::from_secs(1));
            *dynamic_counter.lock() += 1;
        })
        .persist();

    "Channels ensure every value sent is received. Try \
        clicking the button quickly and seeing how the \
        channel version increments for every click while \
        the dynamic version increments at most once every \
        second."
        .and(
            "Click Me"
                .into_button()
                .on_click(move |_| {
                    let now = Instant::now();
                    sender.send(now).expect("value to be received");
                    dynamic_value.set(now);
                })
                .centered(),
        )
        .and(
            "Channel Counter"
                .and(channel_counter_label)
                .into_rows()
                .and("Dynamic Counter".and(dynamic_counter_label).into_rows())
                .into_columns()
                .centered(),
        )
        .into_rows()
        .run()
}
