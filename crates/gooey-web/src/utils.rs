use gooey_core::events::{MouseButtons, MouseEvent};
use gooey_core::math::units::Px;
use gooey_core::math::Point;

#[must_use]
pub fn mouse_event_from_web(event: &web_sys::MouseEvent) -> MouseEvent {
    MouseEvent {
        current_buttons: MouseButtons::multiple(u64::from(event.buttons())),
        button: MouseButtons::single(u8::try_from(event.button()).unwrap_or_default()),
        position: Some(Point {
            x: Px(event.offset_x()),
            y: Px(event.offset_y()),
        }),
    }
}
