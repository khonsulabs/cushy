use cushy::widget::MakeWidget;
use cushy::widgets::layers::Modal;
use cushy::Run;
use cushy::widgets::{Grid};
use cushy::widgets::grid::{GridDimension, GridWidgets};

fn main() -> cushy::Result {
    let modal_1 = Modal::new();
    let button_1 = "Show Modal 1 Message"
        .into_button()
        .on_click({
            let modal = modal_1.clone();
            move |_| modal.message("Modal 1", "Dismiss")
        })
        .align_top();

    let modal_2 = Modal::new();
    let button_2 = "Show Modal 2 Message"
        .into_button()
        .on_click({
            let modal = modal_2.clone();
            move |_| modal.message("Modal 2", "Dismiss")
        })
        .align_top();

    let button_1_container = button_1
        .centered()
        .contain()
        .and(modal_1)
        .into_layers();

    let button_2_container = button_2
        .centered()
        .contain()
        .and(modal_2)
        .into_layers();

    let content = Grid::from_rows(
        GridWidgets::new()
            .and((button_1_container, button_2_container)))
        .dimensions([
            GridDimension::Fractional { weight: 1 },
            GridDimension::Fractional { weight: 1 },
        ]);

    content
        .expand_vertically()
        .run()
}
