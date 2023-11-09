use std::process::exit;

use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::{Button, Expand, Input, Label, Resize, Stack};
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    let valid =
        (&username, &password).map_each(|(username, password)| validate(username, password));

    Resize::width(
        // TODO We need a min/max range for the Resize widget
        Lp::points(400),
        Stack::rows(
            Stack::columns(
                Label::new("Username").and(
                    Input::new(username.clone())
                        .centered()
                        .fit_horizontally()
                        .expand(),
                ),
            )
            .and(Stack::columns(
                Label::new("Password").and(
                    // TODO secure input
                    Input::new(password.clone())
                        .centered()
                        .fit_horizontally()
                        .expand(),
                ),
            ))
            .and(Stack::columns(
                Button::new("Cancel")
                    .on_click(|_| {
                        eprintln!("Login cancelled");
                        exit(0)
                    })
                    .into_escape()
                    .and(Expand::empty())
                    .and(
                        Button::new("Log In")
                            .enabled(valid)
                            .on_click(move |_| {
                                println!("Welcome, {}", username.get());
                                exit(0);
                            })
                            .into_default(),
                    ),
            )),
        ),
    )
    .centered()
    .expand()
    .run()
}

fn validate(username: &String, password: &String) -> bool {
    !username.is_empty() && !password.is_empty()
}
