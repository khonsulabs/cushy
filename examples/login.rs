use std::process::exit;

use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::{Align, Button, Expand, Input, Label, Resize, Stack};
use gooey::{children, Run, WithClone};
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    // TODO This is absolutely horrible. The problem is that within for_each,
    // the value is still locked. Thus, we can't have a generic callback that
    // tries to lock the value that is being mapped in for_each.
    //
    // We might be able to make a genericized implementation for_each for
    // tuples, ie, (&Dynamic, &Dynamic).for_each(|(a, b)| ..).
    let valid = Dynamic::default();
    username.for_each((&valid, &password).with_clone(|(valid, password)| {
        move |username: &String| {
            password.map_ref(|password| valid.update(validate(username, password)))
        }
    }));
    password.for_each((&valid, &username).with_clone(|(valid, username)| {
        move |password: &String| {
            username.map_ref(|username| valid.update(validate(username, password)))
        }
    }));

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
                Button::new("Cancel").on_click(|_| exit(0)).into_escape(),
                Expand::empty(),
                Button::new("Log In")
                    .on_click(move |_| {
                        if valid.get() {
                            println!("Welcome, {}", username.get());
                            exit(0);
                        } else {
                            eprintln!("Enter a username and password")
                        }
                    })
                    .into_default(), // TODO enable/disable based on valid
            ]),
        ]),
    )))
    .run()
}

fn validate(username: &String, password: &String) -> bool {
    !username.is_empty() && !password.is_empty()
}
