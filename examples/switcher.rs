use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, WidgetInstance};
use gooey::widgets::{Button, Label, Switcher};
use gooey::Run;

#[derive(Debug)]
enum ActiveContent {
    Intro,
    Success,
}

fn main() -> gooey::Result {
    let active = Dynamic::new(ActiveContent::Intro);

    Switcher::new(active.clone(), move |content| match content {
        ActiveContent::Intro => intro(active.clone()),
        ActiveContent::Success => success(active.clone()),
    })
    .contain()
    .centered()
    .expand()
    .run()
}

fn intro(active: Dynamic<ActiveContent>) -> WidgetInstance {
    const INTRO: &str = "This example demonstrates the Switcher<T> widget, which uses a mapping function to convert from a generic type to the widget it uses for its contents.";
    Label::new(INTRO)
        .and(
            Button::new("Switch!")
                .on_click(move |_| active.set(ActiveContent::Success))
                .centered(),
        )
        .into_rows()
        .make_widget()
}

fn success(active: Dynamic<ActiveContent>) -> WidgetInstance {
    Label::new("The value changed to `ActiveContent::Success`!")
        .and(
            Button::new("Start Over")
                .on_click(move |_| active.set(ActiveContent::Intro))
                // .centered(),
        )
        .into_rows()
        .make_widget()
}
