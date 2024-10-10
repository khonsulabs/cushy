use cushy::kludgine::app::winit::window::CursorIcon;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::input::InputValue;
use cushy::widgets::Custom;
use cushy::Run;

fn main() -> cushy::Result {
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
