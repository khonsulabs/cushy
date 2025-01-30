//! Shows the ability to share widgets between multiple windows.
//!
//! This example was created to test a fix for
//! <https://github.com/khonsulabs/cushy/issues/139>. The issue was that if the
//! same Switcher widget was shown on two separate windows, only one window
//! would unmount the existing widget.
//!
//! When running this example after the bug has been fixed, unmounted messages
//! should be printed twice: once per each window.
use cushy::reactive::value::{Dynamic, Switchable};
use cushy::widget::MakeWidget;
use cushy::widgets::Custom;
use cushy::{Open, PendingApp};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Contents {
    A,
    B,
}

#[cushy::main]
fn main(app: &mut PendingApp) -> cushy::Result {
    let selected = Dynamic::new(Contents::A);

    // Open up another window containing our controls
    selected
        .new_radio(Contents::A)
        .labelled_by("A")
        .and(selected.new_radio(Contents::B).labelled_by("B"))
        .into_rows()
        .open(app)?;

    let display = selected
        .switcher(|contents, _| match contents {
            Contents::A => Custom::new("A")
                .on_unmounted(|_| {
                    println!("A unmounted");
                })
                .make_widget(),
            Contents::B => Custom::new("B")
                .on_unmounted(|_| {
                    println!("B unmounted");
                })
                .make_widget(),
        })
        .make_widget();

    // Open two windows with the same switcher instance
    display.to_window().open(app)?;
    display.to_window().open(app)?;

    Ok(())
}
