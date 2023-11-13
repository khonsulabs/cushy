use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, WidgetInstance};
use gooey::widgets::{Button, Label};
use gooey::window::ThemeMode;
use gooey::Run;

fn main() -> gooey::Result {
    let theme_mode = Dynamic::default();
    set_of_containers(1, theme_mode.clone())
        .centered()
        .into_window()
        .with_theme_mode(theme_mode)
        .run()
}

fn set_of_containers(repeat: usize, theme_mode: Dynamic<ThemeMode>) -> WidgetInstance {
    let inner = if let Some(remaining_iters) = repeat.checked_sub(1) {
        set_of_containers(remaining_iters, theme_mode)
    } else {
        Button::new("Toggle Theme Mode")
            .on_click(move |_| {
                theme_mode.map_mut(|mode| mode.toggle());
            })
            .make_widget()
    };
    Label::new("Lowest")
        .and(
            Label::new("Low")
                .and(
                    Label::new("Mid")
                        .and(
                            Label::new("High")
                                .and(Label::new("Highest").and(inner).into_rows().contain())
                                .into_rows()
                                .contain(),
                        )
                        .into_rows()
                        .contain(),
                )
                .into_rows()
                .contain(),
        )
        .into_rows()
        .contain()
        .make_widget()
}
