use gooey_core::events::{MouseButtons, MouseEvent};
use gooey_core::math::units::Px;
use gooey_core::math::Point;

pub(crate) fn mouse_event_from_web(event: web_sys::MouseEvent) -> MouseEvent {
    MouseEvent {
        current_buttons: MouseButtons::multiple(event.buttons() as u64),
        button: MouseButtons::single(event.button() as u8),
        position: Some(Point {
            x: Px(event.offset_x()),
            y: Px(event.offset_y()),
        }),
    }
}
