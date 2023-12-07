use std::panic::UnwindSafe;

use gooey::widget::{MakeWidget, MakeWidgetWithId, WidgetTag};
use gooey::widgets::layers::{OverlayBuilder, OverlayLayer};
use gooey::Run;
use kludgine::Color;
use rand::{thread_rng, Rng};

fn main() -> gooey::Result {
    let overlay = OverlayLayer::default();

    test_widget(&overlay, true)
        .centered()
        .and(overlay)
        .into_layers()
        .run()
}

fn test_widget(overlay: &OverlayLayer, is_root: bool) -> impl MakeWidget {
    let (my_tag, my_id) = WidgetTag::new();
    let right = show_overlay_button("Right", overlay, move |overlay| overlay.right_of(my_id));
    let left = show_overlay_button("Left", overlay, move |overlay| overlay.left_of(my_id));
    let up = show_overlay_button("Up", overlay, move |overlay| overlay.above(my_id));
    let down = show_overlay_button("Down", overlay, move |overlay| overlay.below(my_id));

    let mut buttons = up
        .centered()
        .and(left.and(right).into_columns())
        .and(down.centered())
        .into_rows()
        .contain();

    if !is_root {
        buttons = buttons.background_color(Color::new(
            thread_rng().gen(),
            thread_rng().gen(),
            thread_rng().gen(),
            255,
        ))
    }

    buttons.pad().make_with_id(my_tag)
}

fn show_overlay_button(
    label: &str,
    overlay: &OverlayLayer,
    direction_func: impl for<'a> Fn(OverlayBuilder<'a>) -> OverlayBuilder<'a>
        + Send
        + UnwindSafe
        + 'static,
) -> impl MakeWidget {
    let overlay = overlay.clone();
    label.into_button().on_click(move |()| {
        direction_func(overlay.build_overlay(test_widget(&overlay, false)))
            .hide_on_unhover()
            .show()
            .forget();
    })
}
