use cushy::kludgine::app::winit::window::Fullscreen;
use cushy::kludgine::app::Monitor;
use cushy::reactive::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetList};
use cushy::{App, Open};

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {
    let monitors = app.monitors().expect("monitors api not supported");
    let fullscreen = Dynamic::new(None);

    fullscreen
        .new_radio(None)
        .labelled_by("Not Fullscreen")
        .and(
            monitors
                .available
                .iter()
                .enumerate()
                .map(|(index, monitor)| monitor_modes(index, monitor, &fullscreen))
                .collect::<WidgetList>()
                .into_rows(),
        )
        .into_rows()
        .pad()
        .vertical_scroll()
        .expand()
        .into_window()
        .fullscreen(fullscreen)
        .open(app)?;
    Ok(())
}

fn monitor_modes(
    index: usize,
    monitor: &Monitor,
    fullscreen: &Dynamic<Option<Fullscreen>>,
) -> WidgetList {
    let name = monitor.name().unwrap_or_else(|| format!("Monitor {index}"));

    name.h1()
        .and(
            fullscreen
                .new_radio(Some(Fullscreen::Borderless(Some(monitor.handle().clone()))))
                .labelled_by("Borderless Fullscreen"),
        )
        .chain(monitor.video_modes().map(|mode| {
            fullscreen
                .new_radio(Some(Fullscreen::Exclusive(mode.handle().clone())))
                .labelled_by(format!(
                    "{}x{} @ {}Hz ({}-bit color)",
                    mode.size().width,
                    mode.size().height,
                    mode.refresh_rate_millihertz() as f32 / 1_000.,
                    mode.bit_depth()
                ))
        }))
}
