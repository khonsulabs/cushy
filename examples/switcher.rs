use cushy::value::{Destination, Dynamic, Switchable};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::Run;

#[derive(Debug, Eq, PartialEq)]
enum ActiveContent {
    Intro,
    Success,
}

fn main() -> cushy::Result {
    let active = Dynamic::new(ActiveContent::Intro);

    active
        .switcher(|current, active| match current {
            ActiveContent::Intro => intro(active.clone()),
            ActiveContent::Success => success(active.clone()),
        })
        .contain()
        .centered()
        .run()
}

fn intro(active: Dynamic<ActiveContent>) -> WidgetInstance {
    const INTRO: &str = "This example demonstrates the Switcher<T> widget, which uses a mapping function to convert from a generic type to the widget it uses for its contents.";
    INTRO
        .and(
            "Switch!"
                .into_button()
                .on_click(move |_| active.set(ActiveContent::Success))
                .centered(),
        )
        .into_rows()
        .make_widget()
}

fn success(active: Dynamic<ActiveContent>) -> WidgetInstance {
    "The value changed to `ActiveContent::Success`!"
        .and(
            "Start Over"
                .into_button()
                .on_click(move |_| active.set(ActiveContent::Intro))
                .centered(),
        )
        .into_rows()
        .make_widget()
}
