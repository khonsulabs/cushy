use std::process::exit;

use gooey::value::{Dynamic, Validations};
use gooey::widget::MakeWidget;
use gooey::widgets::input::{InputValue, MaskedString};
use gooey::widgets::Expand;
use gooey::Run;
use kludgine::figures::units::Lp;

fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();
    let validations = Validations::default();

    let username_valid = validations.validate(&username, |u: &String| {
        if u.is_empty() {
            Err("usernames must contain at least one character")
        } else {
            Ok(())
        }
    });

    let password_valid = validations.validate(&password, |u: &MaskedString| match u.len() {
        0..=7 => Err("passwords must be at least 8 characters long"),
        _ => Ok(()),
    });

    // TODO this should be a grid layout to ensure proper visual alignment.
    let username_field = "Username"
        .align_left()
        .and(
            username
                .clone()
                .into_input()
                .placeholder("Username")
                .validation(username_valid)
                .hint("* required"),
        )
        .into_rows();

    let password_field = "Password"
        .align_left()
        .and(
            password
                .clone()
                .into_input()
                .placeholder("Password")
                .validation(password_valid)
                .hint("* required, 8 characters min"),
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
                .on_click(validations.when_valid(move |()| {
                    println!("Welcome, {}", username.get());
                    exit(0);
                }))
                .into_default(),
        )
        .into_columns();

    username_field
        .and(password_field)
        .and(buttons)
        .into_rows()
        .contain()
        .width(Lp::inches(3)..Lp::inches(6))
        .pad()
        .scroll()
        .centered()
        .run()
}
