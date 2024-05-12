use std::time::Duration;

use cushy::value::{Destination, Dynamic};
use cushy::widget::MakeWidget;
use cushy::widgets::progress::Progressable;
use cushy::{Open, PendingApp, TokioRuntime};
use tokio::time::sleep;

fn main() {
    let app = PendingApp::new(TokioRuntime::default());
    let progress = Dynamic::new(0_u8);
    let progress_bar = progress.clone().progress_bar();
    "Press Me"
        .into_button()
        .on_click(move |_| {
            tokio::spawn(do_something(progress.clone()));
        })
        .and(progress_bar)
        .into_rows()
        .centered()
        .expand()
        .run_in(app)
        .expect("error starting Cushy");
}

async fn do_something(progress: Dynamic<u8>) {
    for i in 0..u8::MAX {
        progress.set(i);
        sleep(Duration::from_millis(10)).await
    }
}
