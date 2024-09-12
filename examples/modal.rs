use cushy::widget::MakeWidget;
use cushy::widgets::layers::Modal;
use cushy::Run;

fn main() -> cushy::Result {
    let modal = Modal::new();

    "Show Modal"
        .into_button()
        .on_click({
            let modal = modal.clone();
            move |_| modal.message("This is a modal", "Dismiss")
        })
        .align_top()
        .and(modal)
        .into_layers()
        .run()
}
