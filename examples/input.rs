use gooey::value::StringValue;
use gooey::widget::MakeWidget;
use gooey::Run;

fn main() -> gooey::Result {
    "Hello".into_input().expand().run()
}
