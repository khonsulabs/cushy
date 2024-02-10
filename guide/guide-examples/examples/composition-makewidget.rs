use cushy::value::{Dynamic, IntoValue, Value};
use cushy::widgets::input::InputValue;

fn composition_makewidget() -> impl cushy::widget::MakeWidget {
    // ANCHOR: definition
    use cushy::widget::{MakeWidget, WidgetInstance};

    struct FormField {
        label: Value<String>,
        field: WidgetInstance,
    }

    impl FormField {
        pub fn new(label: impl IntoValue<String>, field: impl MakeWidget) -> Self {
            Self {
                label: label.into_value(),
                field: field.make_widget(),
            }
        }
    }
    // ANCHOR_END: definition

    // ANCHOR: makewidget
    impl MakeWidget for FormField {
        fn make_widget(self) -> WidgetInstance {
            self.label
                .align_left()
                .and(self.field)
                .into_rows()
                .make_widget()
        }
    }

    FormField::new(
        "Label",
        Dynamic::<String>::default()
            .into_input()
            .placeholder("Field"),
    )
    // ANCHOR_END: makewidget
}

fn main() {
    guide_examples::book_example!(composition_makewidget).untested_still_frame();
}

#[test]
fn runs() {
    main();
}
