use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, WidgetInstance};
use gooey::widgets::container::ContainerShadow;
use gooey::window::ThemeMode;
use gooey::Run;
use kludgine::figures::units::Lp;
use kludgine::figures::Point;

fn main() -> gooey::Result {
    let theme_mode = Dynamic::default();
    set_of_containers(3, theme_mode.clone())
        .centered()
        .into_window()
        .themed_mode(theme_mode)
        .run()
}

fn set_of_containers(repeat: usize, theme_mode: Dynamic<ThemeMode>) -> WidgetInstance {
    let inner = if let Some(remaining_iters) = repeat.checked_sub(1) {
        set_of_containers(remaining_iters, theme_mode)
    } else {
        "Toggle Theme Mode"
            .into_button()
            .on_click(move |_| {
                theme_mode.map_mut(|mode| mode.toggle());
            })
            .make_widget()
    };
    "Lowest"
        .and(
            "Low"
                .and(
                    "Mid"
                        .and(
                            "High"
                                .and(
                                    "Highest"
                                        .and(inner)
                                        .into_rows()
                                        .contain()
                                        .shadow(drop_shadow()),
                                )
                                .into_rows()
                                .contain()
                                .shadow(drop_shadow()),
                        )
                        .into_rows()
                        .contain()
                        .shadow(drop_shadow()),
                )
                .into_rows()
                .contain()
                .shadow(drop_shadow()),
        )
        .into_rows()
        .contain()
        .shadow(drop_shadow())
        .make_widget()
}

fn drop_shadow() -> ContainerShadow<Lp> {
    ContainerShadow::new(Point::new(Lp::ZERO, Lp::mm(1))).spread(Lp::mm(1))
}
