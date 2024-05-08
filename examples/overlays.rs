use cushy::widget::{MakeWidget, MakeWidgetWithTag, WidgetTag};
use cushy::widgets::container::ContainerShadow;
use cushy::widgets::layers::{OverlayBuilder, OverlayLayer};
use cushy::Run;
use figures::units::Lp;
use figures::{Point, Zero};
use kludgine::app::winit::event::MouseButton;
use kludgine::Color;
use rand::{thread_rng, Rng};

fn main() -> cushy::Result {
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
        buttons = buttons
            .background_color(Color::new(
                thread_rng().gen(),
                thread_rng().gen(),
                thread_rng().gen(),
                255,
            ))
            .shadow(
                ContainerShadow::new(Point::new(Lp::ZERO, Lp::mm(2)))
                    .blur_radius(Lp::mm(1))
                    .spread(Lp::mm(1)),
            );
    }

    buttons.pad().make_with_tag(my_tag)
}

fn show_overlay_button(
    label: &str,
    overlay: &OverlayLayer,
    direction_func: impl for<'a> Fn(OverlayBuilder<'a>) -> OverlayBuilder<'a> + Send + 'static,
) -> impl MakeWidget {
    let overlay = overlay.clone();
    let button_tag = WidgetTag::unique();
    let button_id = button_tag.id();
    label
        .into_button()
        .on_click(move |click| {
            let overlay = overlay.build_overlay(test_widget(&overlay, false));
            let overlay = match click {
                Some(click) if click.mouse_button == MouseButton::Right => {
                    overlay.above(button_id).at(click.window_location)
                }
                _ => direction_func(overlay),
            };

            overlay.hide_on_unhover().show().forget();
        })
        .make_with_tag(button_tag)
}
