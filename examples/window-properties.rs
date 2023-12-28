use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, WidgetInstance};
use gooey::Run;
use kludgine::figures::Size;

fn main() -> gooey::Result {
    let focused = Dynamic::new(false);
    let occluded = Dynamic::new(false);
    let inner_size = Dynamic::new(Size::default());

    let widgets = focused
        .map_each(|v| format!("focused: {:?}", v))
        .and(occluded.map_each(|v| format!("occluded: {:?}", v)))
        .and(inner_size.map_each(|v| format!("inner_size: {:?}", v)))
        .into_rows()
        .centered();

    gooey::window::Window::<WidgetInstance>::for_widget(widgets)
        .focused(focused)
        .occluded(occluded)
        .inner_size(inner_size)
        .run()
}
