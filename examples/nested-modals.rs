use cushy::dialog::MessageBox;
use cushy::widget::MakeWidget;
use cushy::widgets::layers::{Modal, ModalTarget};
use cushy::Run;

fn main() -> cushy::Result {
    let modal = Modal::new();

    "Show Modal"
        .into_button()
        .on_click({
            let modal = modal.clone();
            move |_| show_modal(&modal, 1)
        })
        .align_top()
        .pad()
        .and(modal)
        .into_layers()
        .run()
}

fn show_modal(present_in: &impl ModalTarget, level: usize) {
    let handle = present_in.new_handle();
    handle
        .build_dialog(
            format!("Modal level: {level}")
                .and("Go Deeper".into_button().on_click({
                    let handle = handle.clone();
                    move |_| {
                        show_modal(&handle, level + 1);
                    }
                }))
                .and("Show message".into_button().on_click({
                    let handle = handle.clone();
                    move |_| {
                        MessageBox::message("This is a MessageBox shown above a modal")
                            .open(&handle);
                    }
                }))
                .into_rows(),
        )
        .with_default_button("Close", || {})
        .with_cancel_button("Close All", {
            let handle = handle.clone();
            move || {
                handle.layer().dismiss();
            }
        })
        .show();
}
