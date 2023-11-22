use gooey::value::Dynamic;
use gooey::widget::MakeWidget;
use gooey::widgets::checkbox::Checkable;
use gooey::Run;

const EXPLANATION: &str =
    "The collapse widget allows showing and hiding another widget based on a Dynamic<bool>.";

fn main() -> gooey::Result {
    let collapse = Dynamic::new(false);

    collapse
        .clone()
        .into_checkbox("Collapse")
        .and(
            "Content Above"
                .contain()
                .and(EXPLANATION.collapse_vertically(collapse))
                .and("Content Below".contain())
                .into_rows(),
        )
        .into_columns()
        .centered()
        .expand()
        .run()
}
