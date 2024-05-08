use cushy::kludgine::include_texture;
use cushy::value::{Destination, Dynamic, MapEach};
use cushy::widget::MakeWidget;
use cushy::widgets::Image;
use cushy::window::PendingWindow;
use cushy::{Application, Open, PendingApp, Run};
use kludgine::LazyTexture;

fn main() -> cushy::Result {
    // To open multiple applications, we need a handle to the application. This
    // starts with the `PendingApp` type.
    let mut app = PendingApp::default();
    // Cushy ensures it's easy to share resources between windows.
    let texture = include_texture!("assets/ferris-happy.png").expect("valid image");

    let open_windows = Dynamic::new(0_usize);
    let counter = Dynamic::new(0_usize);

    // We're going to open two windows as part of the app startup process.
    // First, the "main" window that displays some stats about the open windows.
    (&open_windows, &counter)
        .map_each(|(open, counter)| {
            format!(
                "There are {open} other window(s) open. {counter} total windows have been opened"
            )
        })
        .and(Image::new(texture.clone()))
        .and(open_window_button(&app, &open_windows, &counter, &texture))
        .into_rows()
        .centered()
        // The other examples call run() on the widget/window. Since we're
        // opening two windows at the app's startup,
        .open(&mut app)?;

    // And now let's open our first "clone" window -- the window that clicking
    // the open button on any of the windows will create.
    open_another_window(&mut app, &open_windows, &counter, &texture);

    // Run the application
    app.run()
}

/// Returns a button that invokes `open_another_window` when clicked.
fn open_window_button(
    app: &impl Application,
    open_windows: &Dynamic<usize>,
    counter: &Dynamic<usize>,
    texture: &LazyTexture,
) -> impl MakeWidget {
    let mut app = app.as_app();
    let open_windows = open_windows.clone();
    let counter = counter.clone();
    let texture = texture.clone();
    "Open Another Window".into_button().on_click(move |_| {
        open_another_window(&mut app, &open_windows, &counter, &texture);
    })
}

/// Opens another window that contains a button that opens another window.
fn open_another_window(
    app: &mut impl Application,
    open_windows: &Dynamic<usize>,
    counter: &Dynamic<usize>,
    texture: &LazyTexture,
) {
    let my_number = counter.map_mut(|mut count| {
        *count += 1;
        *count
    });

    let open_windows = open_windows.clone();
    open_windows.map_mut(|mut open_windows| *open_windows += 1);

    let window = PendingWindow::default();
    let handle = window.handle();

    window
        .with_root(
            format!("This is window {my_number}")
                .and(open_window_button(app, &open_windows, counter, texture))
                .and(Image::new(texture.clone()))
                .and(
                    "Close"
                        .into_button()
                        .on_click(move |_| handle.request_close()),
                )
                .into_rows()
                .centered(),
        )
        .on_close(move || open_windows.map_mut(|mut open_windows| *open_windows -= 1))
        .open(app)
        .expect("error opening another window");
}
