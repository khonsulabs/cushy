use std::time::Duration;

use gooey::animation::{Animation, AnimationHandle, Spawn};
use gooey::value::Dynamic;
use gooey::widgets::{Button, Label, Stack};
use gooey::{widgets, Run, WithClone};

fn main() -> gooey::Result {
    let animation = Dynamic::new(AnimationHandle::new());
    let value = Dynamic::new(50);
    let label = value.map_each(|value| value.to_string());
    Stack::columns(widgets![
        Button::new("To 0").on_click(animate_to(&animation, &value, 0)),
        Label::new(label),
        Button::new("To 100").on_click(animate_to(&animation, &value, 100)),
    ])
    .run()
}

fn animate_to(
    animation: &Dynamic<AnimationHandle>,
    value: &Dynamic<u8>,
    target: u8,
) -> impl FnMut(()) {
    (animation, value).with_clone(|(animation, value)| {
        move |_| {
            animation.set(Animation::linear(value.clone(), target, Duration::from_secs(1)).spawn())
        }
    })
}
