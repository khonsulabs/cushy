use cushy::widget::MakeWidget;
use cushy::widgets::layers::Modal;
use cushy::Run;

fn main() -> cushy::Result {
    let modal = Modal::new();

    "Show Modal"
        .into_button()
        .on_click({
            let modal = modal.clone();
            move |_| {
                modal.present(dialog(&modal));
            }
        })
        .align_top()
        .and(modal)
        .into_layers()
        .run()
}

fn dialog(modal: &Modal) -> impl MakeWidget {
    let modal = modal.clone();
    "This is a modal"
        .and("Dismiss".into_button().on_click(move |_| {
            modal.dismiss();
        }))
        .into_rows()
        .contain()
}
