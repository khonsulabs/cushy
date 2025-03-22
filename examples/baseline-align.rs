use cushy::reactive::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::checkbox::Checkable;
use cushy::widgets::grid::{GridSection, GridWidgets};
use cushy::widgets::Grid;
use cushy::Run;

fn main() -> cushy::Result {
    "Label"
        .and("Button".into_button())
        .and(
            Dynamic::<bool>::default()
                .into_checkbox()
                .labelled_by("Checkbox"),
        )
        .into_columns()
        .contain()
        .and(
            Grid::from_rows(
                GridWidgets::from(GridSection::from(("Label", "Button".into_button()))).and((
                    "Label",
                    Dynamic::<bool>::default()
                        .into_checkbox()
                        .labelled_by("Checkbox"),
                )),
            )
            .contain(),
        )
        .into_rows()
        .centered()
        .run()
}
