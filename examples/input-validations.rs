use cushy::reactive::value::{Dynamic, Source, Validations};
use cushy::reactive::Unwrapped;
use cushy::widget::{MakeWidget, HANDLED, IGNORED};
use cushy::widgets::input::InputValue;
use cushy::widgets::label::Displayable;
use cushy::Run;
use kludgine::app::winit::keyboard::Key;

fn main() -> cushy::Result {
    // The recommended way to do text input validation is to allow the user to
    // input whatever they want, and display a validation message when the input
    // is not valid.
    let first_label = "The text input below allows all input but displays an error if the contents can't be parsed into a usize.";
    let string1 = Dynamic::new(String::new());
    let validations = Validations::default();
    let parsed = string1.map_each(|string1| string1.trim().parse::<usize>());
    let value1 = parsed.unwrapped();
    let input1 = string1
        .to_input()
        .validation(validations.validate_result(parsed.clone()));

    // Another way some users may want to handle input validation is to try to
    // restrict the user from inputting invalid values. `input2` overrides key
    // handling for the input to verify the input is valid. For this example, an
    // arbitrary limit of 10 digits rather than trying to determine if the
    // character inserted would cause the number to be larger than the maximum
    // usize.
    //
    // Restricting user input can be challenging due to the variety of keyboard
    // input expected by the user to be allowed.
    let second_label = "The text input below prevents typing characters that aren't numeric, and only allows 10 digits.";
    let string2 = Dynamic::new(String::new());
    let input2 = string2.to_input().on_key({
        let string1 = string2.clone();
        move |key| match key.logical_key {
            // Allow all ascii digits up to 10 digits in length
            Key::Character(ch)
                if ch.chars().all(|c| c.is_ascii_digit()) && string1.map_ref(String::len) < 10 =>
            {
                IGNORED
            }
            // Allow ascii control characters for text navigation and deletion.
            key if key
                .to_text()
                .is_some_and(|text| text.chars().all(|c| !c.is_ascii_control())) =>
            {
                HANDLED
            }
            _ => IGNORED,
        }
    });

    // There is still a possibilty for `string2` to be unparsable, so we unwrap
    // with 0 if that happens.
    let value2 = string2.map_each(|text| text.parse::<usize>().unwrap_or(0));

    first_label
        .and(
            input1
                .expand()
                .and(value1.into_label().expand())
                .into_columns(),
        )
        .and(second_label)
        .and(
            input2
                .expand()
                .and(value2.into_label().expand())
                .into_columns(),
        )
        .into_rows()
        .centered()
        .run()
}
