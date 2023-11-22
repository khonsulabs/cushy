use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::button::ButtonKind;
use gooey::Run;

fn main() -> gooey::Result {
    let clicked_label = Dynamic::new(String::from("Click a Button"));
    let default_is_outline = Dynamic::new(false);
    let default_button_style = default_is_outline.map_each(|is_outline| {
        if *is_outline {
            ButtonKind::Outline
        } else {
            ButtonKind::Solid
        }
    });

    clicked_label
        .clone()
        .and(
            "Normal Button"
                .into_button()
                .on_click(
                    clicked_label.with_clone(|label| {
                        move |_| label.set(String::from("Clicked Normal Button"))
                    }),
                )
                .and(
                    "Outline Button"
                        .into_button()
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Outline Button"))
                        }))
                        .kind(ButtonKind::Outline),
                )
                .and(
                    "Transparent Button"
                        .into_button()
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Transparent Button"))
                        }))
                        .kind(ButtonKind::Transparent),
                )
                .and(
                    "Default Button"
                        .into_button()
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Default Button"))
                        }))
                        .kind(default_button_style)
                        .into_default(),
                )
                .and("Set Default to Outline".into_checkbox(default_is_outline))
                .into_columns(),
        )
        .into_rows()
        .centered()
        .expand()
        .run()
}
