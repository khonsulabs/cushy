use std::process::exit;

use gooey::value::{Dynamic, MapEach};
use gooey::widget::MakeWidget;
use gooey::widgets::input::{InputValue, MaskedString};
use gooey::widgets::Expand;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    let valid =
        (&username, &password).map_each(|(username, password)| validate(username, password));

    // TODO this should be a grid layout to ensure proper visual alignment.
    let username_field = "Username"
        .align_left()
        .and(username.clone().into_input())
        .into_rows();

    let password_field = "Password"
        .align_left()
        .and(
            // TODO secure input
            password.clone().into_input(),
        )
        .into_rows();

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
                .on_click(move |_| {
                    println!("Welcome, {}", username.get());
                    exit(0);
                })
                .into_default()
                .with_enabled(valid),
        )
        .into_columns();

    username_field
        .pad()
        .and(password_field.pad())
        .and(buttons.pad())
        .into_rows()
        .contain()
        .width(Lp::inches(3)..Lp::inches(6))
        .pad()
        .scroll()
        .centered()
        .expand()
        .run()
}

fn validate(username: &String, password: &MaskedString) -> bool {
    !username.is_empty() && !password.is_empty()
}
