// ANCHOR: example
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::input::{Input, InputValue};
use cushy::Run;

fn main() -> cushy::Result {
    // Create storage for user to enter a name.
    let name: Dynamic<String> = Dynamic::default();

    // Create our label by using `map_each` to format the name, first checking
    // if it is empty.
    let greeting = name.map_each(|name| {
        let name = if name.is_empty() { "World" } else { name };
        format!("Hello, {name}!")
    });

    // Create the input widget with a placeholder.
    let name_input: Input = name.into_input().placeholder("Name");

    // Stack our widgets as rows, and run the app.
    name_input.and(greeting).into_rows().run()
}
// ANCHOR_END: example

#[test]
fn book() {
    use std::time::Duration;

    fn intro() -> impl MakeWidget {
        let subject: Dynamic<String> = Dynamic::default();
        let greeting: Dynamic<String> = subject.map_each(|subject| {
            let subject = if subject.is_empty() { "World" } else { subject };
            format!("Hello, {subject}!")
        });

        let name_input: Input = subject.into_input().placeholder("Name");

        name_input.and(greeting).into_rows()
    }

    cushy::example!(intro).animated(|animation| {
        animation.wait_for(Duration::from_secs(1)).unwrap();
        animation
            .animate_text_input("Ferris 🦀", Duration::from_secs(1))
            .unwrap();
        animation.wait_for(Duration::from_secs(1)).unwrap();
    });
}
