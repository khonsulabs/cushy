use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::Run;

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Choice {
    #[default]
    A,
    B,
    C,
}

fn main() -> gooey::Result {
    let option = Dynamic::default();

    option
        .new_radio(Choice::A, "A")
        .and(option.new_radio(Choice::B, "B"))
        .and(option.new_radio(Choice::C, "C"))
        .into_rows()
        .centered()
        .run()
}
