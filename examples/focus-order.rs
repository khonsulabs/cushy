use std::process::exit;

use gooey::value::{Dynamic, MapEach, StringValue};
use gooey::widget::{MakeWidget, MakeWidgetWithId, WidgetTag};
use gooey::widgets::Expand;
use gooey::Run;
use kludgine::figures::units::Lp;

/// This example is the same as login, but it has an explicit tab order to
/// change from the default order (username, password, cancel, log in) to
/// username, password, log in, cancel.
fn main() -> gooey::Result {
    let username = Dynamic::default();
    let password = Dynamic::default();

    let valid =
        (&username, &password).map_each(|(username, password)| validate(username, password));

    let (login_tag, login_id) = WidgetTag::new();
    let (cancel_tag, cancel_id) = WidgetTag::new();
    let (username_tag, username_id) = WidgetTag::new();

    // TODO this should be a grid layout to ensure proper visual alignment.
    let username_row = "Username"
        .and(
            username
                .clone()
                .into_input()
                .make_with_id(username_tag)
                .expand(),
        )
        .into_columns();

    let password_row = "Password"
        .and(
            // TODO secure input
            password
                .clone()
                .into_input()
                .with_next_focus(login_id)
                .expand(),
        )
        .into_columns();

    let buttons = "Cancel"
        .into_button()
        .on_click(|_| {
            eprintln!("Login cancelled");
            exit(0)
        })
        .make_with_id(cancel_tag)
        .into_escape()
        .with_next_focus(username_id)
        .and(Expand::empty())
        .and(
            "Log In"
                .into_button()
                .enabled(valid)
                .on_click(move |_| {
                    println!("Welcome, {}", username.get());
                    exit(0);
                })
                .make_with_id(login_tag)
                .into_default()
                .with_next_focus(cancel_id),
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
