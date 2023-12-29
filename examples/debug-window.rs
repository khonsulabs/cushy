use cushy::debug::DebugContext;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::{Application, Open, PendingApp};

const INTRO: &str = "This example demonstrates the DebugContext, which allows observing values easily throughout GUI";

fn main() -> cushy::Result {
    let app = PendingApp::default();
    let dbg = DebugContext::default();
    let window_count = Dynamic::new(0_usize);
    let total_windows = Dynamic::new(0_usize);

    dbg.observe("Open Windows", &window_count);
    dbg.observe("Total Windows", &total_windows);
    dbg.clone().open(&app)?;

    INTRO
        .and("Open a Window".into_button().on_click({
            let app = app.as_app();

            move |()| open_a_window(&window_count, &total_windows, &dbg, &app)
        }))
        .into_rows()
        .centered()
        .run_in(app)
}

fn open_a_window(
    window_count: &Dynamic<usize>,
    total_windows: &Dynamic<usize>,
    dbg: &DebugContext,
    app: &dyn Application,
) {
    *window_count.lock() += 1;
    let window_number = total_windows.map_mut(|total| {
        *total += 1;
        *total
    });
    let window_title = format!("Window #{window_number}");
    let dbg = dbg.section(&window_title);

    let value = Dynamic::new(0_u8);
    dbg.observe("Slider", &value);

    let window_count = window_count.clone();
    let _ = format!("This is window {window_number}.")
        .and(value.slider())
        .into_rows()
        .centered()
        .into_window()
        .titled(window_title)
        .on_close(move || {
            *window_count.lock() -= 1;
        })
        .open(app);
}
