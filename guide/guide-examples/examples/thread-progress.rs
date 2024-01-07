use std::time::Duration;

use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::progress::Progressable;
use guide_examples::book_example;

fn thread_progress() -> impl MakeWidget {
    // ANCHOR: example
    let progress = Dynamic::new(0_u8);
    std::thread::spawn({
        let progress = progress.clone();
        move || {
            while progress.get() < 10 {
                std::thread::sleep(Duration::from_millis(100));
                progress.set(progress.get() + 1);
            }
        }
    });

    progress.progress_bar_to(10)
    // ANCHOR_END: example
}

fn main() {
    book_example!(thread_progress).animated(|recorder| {
        recorder.wait_for(Duration::from_secs(2)).unwrap();
    });
}

#[test]
fn runs() {
    main();
}
