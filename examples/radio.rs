use cushy::value::Dynamic;
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
        .new_radio(Choice::A)
        .labelled_by("A")
        .and(option.new_radio(Choice::B).labelled_by("B"))
        .and(option.new_radio(Choice::C).labelled_by("C"))
        .into_rows()
        .centered()
        .run()
}
