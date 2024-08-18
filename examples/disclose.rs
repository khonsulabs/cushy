use cushy::widget::MakeWidget;
use cushy::widgets::Disclose;
use cushy::Run;

fn disclose() -> impl MakeWidget {
    Disclose::new(
        "This is some inner content"
            .align_left()
            .and(Disclose::new("This is even further inside".contain()))
            .into_rows(),
    )
    .labelled_by("This demonstrates the Disclose widget")
}

fn main() -> cushy::Result {
    disclose().run()
}

#[test]
fn runs() {
    use std::time::Duration;

    use cushy::animation::easings::EaseInOutSine;
    use cushy::figures::{Point, Px2D};
    use kludgine::app::winit::event::MouseButton;

    cushy::example!(disclose, 600, 300)
        .prepare_with(|r| {
            r.set_cursor_position(Point::px(16, 64));
            r.set_cursor_visible(true);
        })
        .animated(|r| {
            r.animate_cursor_to(Point::px(30, 30), Duration::from_millis(500), EaseInOutSine)
                .unwrap();
            r.animate_mouse_button(MouseButton::Left, Duration::from_millis(100))
                .unwrap();
            r.wait_for(Duration::from_secs(1)).unwrap();
            r.animate_mouse_button(MouseButton::Left, Duration::from_millis(100))
                .unwrap();
            r.animate_cursor_to(Point::px(16, 64), Duration::from_millis(500), EaseInOutSine)
                .unwrap();
            r.wait_for(Duration::from_secs(1)).unwrap();
        });
}
