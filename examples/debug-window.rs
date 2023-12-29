use cushy::debug::DebugContext;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::{Application, Open, PendingApp};

const INTRO: &str = "This example demonstrates the DebugContext, which allows observing values easily throughout GUI";

fn main() -> cushy::Result {
    let app = PendingApp::default();
    let info = DebugContext::default();

    let window_count = Dynamic::new(0_usize);
    let total_windows = info.dbg("Total Windows", Dynamic::new(0_usize));
    let open_window_button = "Open a Window"
        .into_button()
        .on_click({
            let app = app.as_app();
            let info = info.clone();
            let window_count = window_count.clone();
            let total_windows = total_windows.clone();
            move |()| open_a_window(&window_count, &total_windows, &info, &app)
        })
        .make_widget();

    info.observe("Open Windows", &window_count, |window_count| {
        window_count
            .map_each(ToString::to_string)
            .and(open_window_button.clone())
            .into_columns()
    });

    info.clone().open(&app)?;

    INTRO
        .and(open_window_button)
        .into_rows()
        .centered()
        .run_in(app)
}

fn open_a_window(
    window_count: &Dynamic<usize>,
    total_windows: &Dynamic<usize>,
    info: &DebugContext,
    app: &dyn Application,
) {
    *window_count.lock() += 1;
    let window_number = total_windows.map_mut(|total| {
        *total += 1;
        *total
    });
    let window_title = format!("Window #{window_number}");
    let dbg = info.section(&window_title);

    let value = dbg.dbg("Slider", Dynamic::new(0_u8));

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
