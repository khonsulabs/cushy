use cushy::reactive::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::Run;

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Choice {
    #[default]
    A,
    B,
    C,
}

fn main() -> cushy::Result {
    let option = Dynamic::default();

    option
        .new_select(Choice::A, "A")
        .and(option.new_select(Choice::B, "B"))
        .and(option.new_select(Choice::C, "C"))
        .into_rows()
        .centered()
        .run()
}
