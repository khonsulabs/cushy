use std::time::Duration;

use cushy::animation::easings::EaseInOutSine;
use cushy::widget::MakeWidget;
use figures::units::Px;
use figures::{Point, Size};

fn ui() -> impl MakeWidget {
    "Hello World".into_button().centered()
}

fn main() {
    // The default recorder generated solid, rgb images.
    let mut recorder = ui()
        .build_recorder()
        .size(Size::new(320, 240))
        .finish()
        .unwrap();
    let initial_point = Point::new(Px::new(140), Px::new(150));
    recorder.set_cursor_position(initial_point);
    recorder.refresh().unwrap();
    let mut animation = recorder.record_animated_png(60);
    animation
        .animate_cursor_to(
            Point::new(Px::new(160), Px::new(120)),
            Duration::from_millis(250),
            EaseInOutSine,
        )
        .unwrap();
    animation.wait_for(Duration::from_millis(500)).unwrap();
    animation
        .animate_cursor_to(initial_point, Duration::from_millis(250), EaseInOutSine)
        .unwrap();
    animation.wait_for(Duration::from_millis(500)).unwrap();
    animation
        .write_to("examples/offscreen-animated.png")
        .unwrap();
}
