use gooey::value::{Dynamic, Validations};
use gooey::widget::MakeWidget;
use gooey::widgets::input::InputValue;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let text = Dynamic::default();
    let validations = Validations::default();

    "Hinted"
        .and(
            text.clone()
                .into_input()
                .validation(validations.validate(&text, validate_input))
                .hint("* required"),
        )
        .and("Not Hinted")
        .and(
            text.clone()
                .into_input()
                .validation(validations.validate(&text, validate_input)),
        )
        .and(
            "Submit"
                .into_button()
                .on_click(validations.clone().when_valid(move |()| {
                    println!(
                    "Success! This callback only happens when all associated validations are valid"
                );
                })),
        )
        .and("Reset".into_button().on_click(move |()| {
            let _value = text.take();
            validations.reset();
        }))
        .into_rows()
        .pad()
        .width(Lp::inches(6))
        .centered()
        .run()
}

fn validate_input(input: &String) -> Result<(), &'static str> {
    if input.is_empty() {
        Err("This field cannot be empty")
    } else if input.trim().is_empty() {
        Err("This field must have at least one non-whitespace character")
    } else {
        Ok(())
    }
}
