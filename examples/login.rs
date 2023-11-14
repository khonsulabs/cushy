use std::process::exit;

use gooey::value::{Dynamic, MapEach, StringValue};
use gooey::widget::MakeWidget;
use gooey::widgets::Expand;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    let valid =
        (&username, &password).map_each(|(username, password)| validate(username, password));

    // TODO this should be a grid layout to ensure proper visual alignment.
    let username_row = "Username"
        .and(username.clone().into_input().expand())
        .into_columns();

    let password_row = "Password"
        .and(
            // TODO secure input
            password.clone().into_input().expand(),
        )
        .into_columns();

    let buttons = "Cancel"
        .into_button()
        .on_click(|_| {
            eprintln!("Login cancelled");
            exit(0)
        })
        .into_escape()
        .and(Expand::empty())
        .and(
            "Log In"
                .into_button()
                .enabled(valid)
                .on_click(move |_| {
                    println!("Welcome, {}", username.get());
                    exit(0);
                })
                .into_default(),
        )
        .into_columns();

    username_row
        .pad()
        .and(password_row.pad())
        .and(buttons.pad())
        .into_rows()
        .contain()
        .width(Lp::points(300)..Lp::points(600))
        .scroll()
        .centered()
        .expand()
        .run()
}

fn validate(username: &String, password: &String) -> bool {
    !username.is_empty() && !password.is_empty()
}
