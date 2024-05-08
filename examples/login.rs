use std::process::exit;

use cushy::value::{Dynamic, Source, Validations};
use cushy::widget::MakeWidget;
use cushy::widgets::input::{InputValue, MaskedString};
use cushy::widgets::layers::OverlayLayer;
use cushy::widgets::Expand;
use cushy::Run;
use figures::units::Lp;

fn main() -> cushy::Result {
    let tooltips = OverlayLayer::default();
    let username = Dynamic::default();
    let password = Dynamic::default();
    let validations = Validations::default();

    let username_field = "Username"
        .align_left()
        .and(
            username
                .to_input()
                .placeholder("Username")
                .validation(validations.validate(&username, |u: &String| {
                    if u.is_empty() {
                        Err("usernames must contain at least one character")
                    } else {
                        Ok(())
                    }
                }))
                .hint("* required")
                .tooltip(
                    &tooltips,
                    "If you can't remember your username, that's because this is a demo.",
                ),
        )
        .into_rows();

    let password_field = "Password"
        .align_left()
        .and(
            password
                .to_input()
                .placeholder("Password")
                .validation(
                    validations.validate(&password, |u: &MaskedString| match u.len() {
                        0..=7 => Err("passwords must be at least 8 characters long"),
                        _ => Ok(()),
                    }),
                )
                .hint("* required, 8 characters min")
                .tooltip(&tooltips, "Passwords are always at least 8 bytes long."),
        )
        .into_rows();

    let buttons = "Cancel"
        .into_button()
        .on_click(|_| {
            eprintln!("Login cancelled");
            exit(0)
        })
        .into_escape()
        .tooltip(&tooltips, "This button quits the program")
        .and(Expand::empty())
        .and(
            "Log In"
                .into_button()
                .on_click(validations.when_valid(move |_| {
                    println!("Welcome, {}", username.get());
                    exit(0);
                }))
                .into_default(),
        )
        .into_columns();

    let ui = username_field
        .and(password_field)
        .and(buttons)
        .into_rows()
        .contain()
        .width(Lp::inches(3)..Lp::inches(6))
        .pad()
        .scroll()
        .centered();

    ui.and(tooltips).into_layers().run()
}
