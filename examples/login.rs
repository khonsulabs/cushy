use std::process::exit;

use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::{Align, Button, Expand, Input, Label, Resize, Stack};
use gooey::{children, Run};
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    let valid =
        (&username, &password).map_each(|(username, password)| validate(username, password));

    Expand::new(Align::centered(Resize::width(
        // TODO We need a min/max range for the Resize widget
        Lp::points(400),
        Stack::rows(children![
            Stack::columns(children![
                Label::new("Username"),
                Expand::new(Align::centered(Input::new(username.clone())).fit_horizontally()),
            ]),
            Stack::columns(children![
                Label::new("Password"),
                Expand::new(
                    Align::centered(
                        // TODO secure input
                        Input::new(password.clone())
                    )
                    .fit_horizontally()
                ),
            ]),
            Stack::columns(children![
                Button::new("Cancel")
                    .on_click(|_| {
                        eprintln!("Login cancelled");
                        exit(0)
                    })
                    .into_escape(),
                Expand::empty(),
                Button::new("Log In")
                    .enabled(valid)
                    .on_click(move |_| {
                        println!("Welcome, {}", username.get());
                        exit(0);
                    })
                    .into_default(),
            ]),
        ]),
    )))
    .run()
}

fn validate(username: &String, password: &String) -> bool {
    !username.is_empty() && !password.is_empty()
}
