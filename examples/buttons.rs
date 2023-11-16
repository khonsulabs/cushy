use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::button::ButtonKind;
use gooey::widgets::{Button, Checkbox};
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
            Button::new("Normal Button")
                .on_click(
                    clicked_label.with_clone(|label| {
                        move |_| label.set(String::from("Clicked Normal Button"))
                    }),
                )
                .and(
                    Button::new("Outline Button")
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Outline Button"))
                        }))
                        .kind(ButtonKind::Outline),
                )
                .and(
                    Button::new("Transparent Button")
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Transparent Button"))
                        }))
                        .kind(ButtonKind::Transparent),
                )
                .and(
                    Button::new("Default Button")
                        .on_click(clicked_label.with_clone(|label| {
                            move |_| label.set(String::from("Clicked Default Button"))
                        }))
                        .kind(default_button_style)
                        .into_default(),
                )
                .and(Checkbox::new(default_is_outline, "Set Default to Outline"))
                .into_columns(),
        )
        .into_rows()
        .centered()
        .expand()
        .run()
}
