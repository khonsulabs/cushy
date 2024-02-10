use std::time::Duration;

use cushy::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn};
use cushy::value::{Destination, Dynamic};
use cushy::widget::MakeWidget;
use cushy::widgets::progress::Progressable;
use cushy::{Run, WithClone};
use figures::units::Lp;

fn main() -> cushy::Result {
    let animation = Dynamic::new(AnimationHandle::new());
    let value = Dynamic::new(50);

    // Cushy's animation system supports using a `Duration` as a step in
    // animation to create a delay. This can also be used to call a function
    // after a specified amount of time:
    Duration::from_secs(1)
        .on_complete(|| println!("Cushy animations are neat!"))
        .launch();

    "To 0"
        .into_button()
        .on_click(animate_to(&animation, &value, 0))
        .and(
            value
                .clone()
                .progress_bar_to(100)
                .width(Lp::inches(3))
                .centered(),
        )
        .and(
            "To 100"
                .into_button()
                .on_click(animate_to(&animation, &value, 100)),
        )
        .into_columns()
        .run()
}

fn animate_to(
    animation: &Dynamic<AnimationHandle>,
    value: &Dynamic<u8>,
    target: u8,
) -> impl FnMut(()) {
    (animation, value).with_clone(|(animation, value)| {
        move |_| {
            // Here we use spawn to schedule the animation, which returns an
            // `AnimationHandle`. When dropped, the animation associated with
            // the `AnimationHandle` will be cancelled. The effect is that this
            // line of code will ensure we only keep one animation running at
            // all times in this example, despite how many times the buttons are
            // pressed.
            animation.set(
                value
                    .transition_to(target)
                    .over(Duration::from_secs(1))
                    .spawn(),
            )
        }
    })
}
