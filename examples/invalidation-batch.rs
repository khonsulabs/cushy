//! This example shows how to use a [`InvalidationBatch`] in a background task
//! to synchronize when the user interface is updated/invalidated.
//!
//! This does not prevent the user interface from displaying the intermediate
//! state if it is redrawn for other reasons or by other threads. For example,
//! if the user resizes the window, the window will be redrawn during the
//! resize, and the current values of the dynamic values will be used.
use std::time::Duration;

use cushy::value::{Destination, Dynamic, InvalidationBatch, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::progress::Progressable;
use cushy::widgets::Grid;
use cushy::Run;

// This task will update both `progress_a` and `progress_b` at varying times,
// but the user interface will only be refreshed when the `InvalidationBatch` is
// dropped.
fn background_task(progress_a: Dynamic<u8>, progress_b: Dynamic<u8>) {
    loop {
        InvalidationBatch::batch(|_batch| {
            // This set of operations has a net effect of incrementing
            // progress_a, and adding 5 to progress_b. But the operations are
            // are done by incrementing by ones and twos over the course of
            // 200ms.
            //
            // This is a convoluted way to simulate having a complex operation
            // in a background thread that a user wishes to synchronize the user
            // interface updates to. The specific operations here aren't
            // important. The important part is that all invalidation related to
            // the changes to these widgets is delayed until this batch is
            // executed, which happens when dropped or by using the batch
            // parameter to this function to invoke them when desired.
            progress_a.set(progress_a.get().wrapping_add(1));
            progress_b.set(progress_b.get().wrapping_add(2));
            std::thread::sleep(Duration::from_millis(100));
            progress_b.set(progress_b.get().wrapping_add(2));
            std::thread::sleep(Duration::from_millis(100));
            progress_a.set(progress_a.get().wrapping_add(1));
            progress_b.set(progress_b.get().wrapping_add(1));
        });
        // We sleep for 300 additional milliseconds to make the average loop
        // take half a second. The progress will only ever be refreshed by this
        // thread when the `progress_a` has been incremented by 2, and
        // `progress_b` has been incremented by 5.
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn main() -> cushy::Result {
    const EXPLANATION: &str = "This example uses a background task that updates these progress values in such a way that it only requests this window be redrawn when the first has been incremented by 2 and the second has been incremented by 5. For fun, try resizing the window to force the window to redraw and observing that the intermediate states can still be seen.";
    let progress_a = Dynamic::new(0);
    let progress_a_text = progress_a.map_each(ToString::to_string);
    let progress_b = Dynamic::new(0);
    let progress_b_text = progress_b.map_each(ToString::to_string);

    std::thread::spawn({
        let progress_a = progress_a.clone();
        let progress_b = progress_b.clone();
        move || background_task(progress_a, progress_b)
    });

    EXPLANATION
        .and(
            Grid::from_rows(
                GridWidgets::new()
                    .and((progress_a.progress_bar(), progress_a_text))
                    .and((progress_b.progress_bar(), progress_b_text)),
            )
            .dimensions([
                GridDimension::Fractional { weight: 1 },
                GridDimension::FitContent,
            ]),
        )
        .into_rows()
        .contain()
        .pad()
        .expand_horizontally()
        .centered()
        .run()
}
