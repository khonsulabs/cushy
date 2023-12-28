use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::input::InputValue;
use gooey::widgets::Custom;
use gooey::Run;
use kludgine::app::winit::window::CursorIcon;

fn main() -> gooey::Result {
    Custom::new(
        "Try hovering the mouse cursor around this window"
            .and(
                Dynamic::new(String::from("Input fields show the text selection cursor"))
                    .into_input(),
            )
            .into_rows()
            .pad()
            .centered(),
    )
    .on_hover(|_location, _context| Some(CursorIcon::Help))
    .on_hit_test(|_location, _context| true)
    .contain()
    .centered()
    .run()
}
