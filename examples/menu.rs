use cushy::widget::MakeWidget;
use cushy::widgets::layers::{OverlayLayer, Overlayable};
use cushy::widgets::menu::{Menu, MenuItem};
use cushy::Run;

#[derive(Clone, Copy, Debug)]
enum MenuOptions {
    First,
    Second,
    Third,
}

fn menu_example() -> impl MakeWidget {
    let overlay = OverlayLayer::default();

    "Click Me"
        .into_button()
        .on_click({
            let overlay = overlay.clone();
            move |click| {
                if let Some(click) = click {
                    menu(true)
                        .overlay_in(&overlay)
                        .at(click.window_location)
                        .show();
                }
            }
        })
        .centered()
        .expand()
        .and(overlay)
        .into_layers()
}

fn main() -> cushy::Result {
    menu_example().run()
}

fn menu(top: bool) -> Menu<MenuOptions> {
    let mut third = MenuItem::build(MenuOptions::Third).text("Third");
    if top {
        third = third.submenu(menu(false));
    }
    Menu::new()
        .on_selected(|selected| {
            println!("Selected item: {selected:?}");
        })
        .with(MenuItem::new(MenuOptions::First, "First"))
        .with(MenuItem::new(MenuOptions::Second, "Second"))
        .with_separator()
        .with(
            MenuItem::build(MenuOptions::Second)
                .text("Disabled")
                .disabled(),
        )
        .with_separator()
        .with(third)
}

#[test]
fn runs() {
    use std::time::Duration;

    use cushy::animation::easings::{EaseInCircular, EaseInOutSine, EaseOutCircular};
    use cushy::figures::{Point, Px2D};
    use kludgine::app::winit::event::MouseButton;

    cushy::example!(menu_example, 800, 600)
        .prepare_with(|r| {
            r.set_cursor_position(Point::px(420, 270));
            r.set_cursor_visible(true);
            r.refresh().unwrap();
        })
        .animated(|r| {
            r.animate_cursor_to(
                Point::px(410, 300),
                Duration::from_millis(500),
                EaseInOutSine,
            )
            .unwrap();
            r.animate_mouse_button(MouseButton::Left, Duration::from_millis(250))
                .unwrap();
            r.wait_for(Duration::from_millis(500)).unwrap();
            r.animate_cursor_to(
                Point::px(430, 325),
                Duration::from_millis(200),
                EaseInCircular,
            )
            .unwrap();
            r.animate_cursor_to(
                Point::px(480, 480),
                Duration::from_millis(400),
                EaseOutCircular,
            )
            .unwrap();
            r.wait_for(Duration::from_millis(300)).unwrap();
            r.animate_cursor_to(
                Point::px(620, 460),
                Duration::from_millis(600),
                EaseInOutSine,
            )
            .unwrap();
            r.animate_cursor_to(
                Point::px(460, 340),
                Duration::from_millis(800),
                EaseInOutSine,
            )
            .unwrap();
            r.animate_mouse_button(MouseButton::Left, Duration::from_millis(250))
                .unwrap();
            r.wait_for(Duration::from_millis(500)).unwrap();
            r.animate_cursor_to(
                Point::px(420, 270),
                Duration::from_millis(500),
                EaseInOutSine,
            )
            .unwrap();
            r.wait_for(Duration::from_millis(500)).unwrap();
        });
}
