use std::time::{SystemTime, UNIX_EPOCH};

use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::widgets::VirtualList;
use cushy::Run;

fn list() -> impl MakeWidget {
    let count = Dynamic::new(50);
    let list = VirtualList::new(&count, |index| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System Time after 1970")
            .as_secs();
        format!("Item {index} - {timestamp}")
    });
    let content_changed = list.content_watcher().clone();

    "Count"
        .and(count.slider_between(0, 10_000).expand_horizontally())
        .and(
            "Refresh"
                .into_button()
                .on_click(move |_| content_changed.notify()),
        )
        .into_columns()
        .and(list.expand())
        .into_rows()
}

fn main() -> cushy::Result {
    list().run()
}

#[test]
fn runs() {
    cushy::example!(list).untested_still_frame();
}
