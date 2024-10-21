use crate::Dynamic;
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};

pub fn create_content(show_on_startup_value: Dynamic<bool>) -> WidgetInstance {
    let home_label = "Home tab content"
        // FIXME remove this alignment, currently labels are center aligned by default.
        .align_left()
        .make_widget();

    let show_on_startup_button= "Show on startup"
        .into_checkbox(show_on_startup_value)
        .make_widget();

    [home_label, show_on_startup_button]
        .into_rows()
        // center all the children, not individually
        .centered()
        .make_widget()
}
