use cushy::kludgine::app::{Monitor, Monitors};
use cushy::widget::{MakeWidget, WidgetInstance, WidgetList};
use cushy::Open;

fn main() -> cushy::Result {
    // Monitor information is only available through winit after the application
    // has started up.
    cushy::run(|app| {
        let monitors = app.monitors();

        "Monitors"
            .h1()
            .and(list_monitors(monitors))
            .into_rows()
            .vertical_scroll()
            .expand()
            .open(app)
            .expect("app running");
    })
}

fn list_monitors(monitors: Option<Monitors>) -> WidgetInstance {
    if let Some(monitors) = monitors {
        monitors
            .available
            .into_iter()
            .enumerate()
            .map(|(index, monitor)| monitor_info(index, monitor, monitors.primary.as_ref()))
            .collect::<WidgetList>()
            .into_rows()
            .make_widget()
    } else {
        "No monitor information available".make_widget()
    }
}

fn monitor_info(index: usize, monitor: Monitor, primary: Option<&Monitor>) -> impl MakeWidget {
    let mut name = monitor
        .name()
        .unwrap_or_else(|| format!("Monitor {}", index + 1));
    if primary.map_or(false, |primary| primary == &monitor) {
        name.push_str(" (Primary)");
    }
    let region = monitor.region();
    let region = format!(
        "{},{} @ {}x{}",
        region.origin.x, region.origin.y, region.size.width, region.size.height
    );

    name.h3().and(region).into_rows().contain()
}
