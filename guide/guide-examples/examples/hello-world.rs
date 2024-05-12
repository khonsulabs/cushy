// ANCHOR: example
use cushy::Run;

fn main() -> cushy::Result {
    "Hello, World!".run()
}
// ANCHOR_END: example

#[test]
fn book() {
    fn hello_world() -> impl cushy::widget::MakeWidget {
        "Hello, World!"
    }

    cushy::example!(hello_world).untested_still_frame();
}
