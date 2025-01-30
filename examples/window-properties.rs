use cushy::figures::units::{Px, UPx};
use cushy::figures::{IntoSigned, Point, Px2D, Size, UPx2D};
use cushy::reactive::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::{App, Open};

#[cushy::main]
fn main(app: &mut App) {
    let focused = Dynamic::new(true);
    let occluded = Dynamic::new(false);
    let maximized = Dynamic::new(false);
    let minimized = Dynamic::new(false);
    let inner_size = Dynamic::new(Size::upx(0, 0));
    let outer_size = Dynamic::new(Size::upx(0, 0));
    let inner_position = Dynamic::new(Point::px(0, 0));
    let outer_position = Dynamic::new(Point::px(0, 0));
    let icon =
        image::load_from_memory(include_bytes!("assets/ferris-happy.png")).expect("valid image");

    let widgets = focused
        .map_each(|v| format!("focused: {:?}", v))
        .and(occluded.map_each(|v| format!("occluded: {:?}", v)))
        .and(maximized.map_each(|v| format!("maximized: {:?}", v)))
        .and(minimized.map_each(|v| format!("minimized: {:?}", v)))
        .and(inner_position.map_each(|v| format!("inner_position: {:?}", v)))
        .and(outer_position.map_each(|v| format!("outer_position: {:?}", v)))
        .and(inner_size.map_each(|v| format!("inner_size: {:?}", v)))
        .and(outer_size.map_each(|v| format!("outer_size: {:?}", v)))
        .and(center_window_button(app, &outer_position, &outer_size))
        .into_rows()
        .centered();

    widgets
        .into_window()
        .focused(focused)
        .occluded(occluded)
        .inner_size(inner_size)
        .outer_size(outer_size)
        .inner_position(inner_position)
        .outer_position(outer_position, true)
        .maximized(maximized)
        .minimized(minimized)
        .icon(Some(icon.into_rgba8()))
        .open(app)
        .expect("app running");
}

fn center_window_button(
    app: &App,
    position: &Dynamic<Point<Px>>,
    outer_size: &Dynamic<Size<UPx>>,
) -> impl MakeWidget {
    "Center window".into_button().on_click({
        let app = app.clone();
        let outer_size = outer_size.clone();
        let position = position.clone();
        move |_| {
            center_window(&app, &position, &outer_size);
        }
    })
}

fn center_window(app: &App, position: &Dynamic<Point<Px>>, outer_size: &Dynamic<Size<UPx>>) {
    let Some(monitors) = app.monitors() else {
        return;
    };
    let Some(monitor) = monitors
        .available
        .iter()
        .find(|m| m.region().contains(position.get()))
        .or(monitors.primary.as_ref())
        .or(monitors.available.first())
    else {
        return;
    };
    let region = monitor.region();
    let window_size = outer_size.get().into_signed();
    let empty_space = region.size - window_size;
    position.set(region.origin + empty_space / 2);
}
