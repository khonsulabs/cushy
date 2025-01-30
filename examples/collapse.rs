use cushy::reactive::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::checkbox::Checkable;
use cushy::Run;

const EXPLANATION: &str =
    "The collapse widget allows showing and hiding another widget based on a Dynamic<bool>.";

fn main() -> cushy::Result {
    let collapse = Dynamic::new(false);

    collapse
        .to_checkbox()
        .labelled_by("Collapse")
        .and(
            "Content Above"
                .contain()
                .and(EXPLANATION.collapse_vertically(collapse))
                .and("Content Below".contain())
                .into_rows(),
        )
        .into_columns()
        .centered()
        .run()
}
