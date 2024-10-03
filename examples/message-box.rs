use cushy::dialog::MessageBox;
use cushy::widget::MakeWidget;
use cushy::widgets::layers::Modal;
use cushy::window::PendingWindow;
use cushy::{App, Open};

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {
    let modal = Modal::new();
    let pending = PendingWindow::default();
    let window = pending.handle();

    pending
        .with_root(
            "Show in Modal layer"
                .into_button()
                .on_click({
                    let modal = modal.clone();
                    move |_| {
                        example_message().open(&modal);
                    }
                })
                .and("Show above window".into_button().on_click({
                    move |_| {
                        example_message().open(&window);
                    }
                }))
                .and("Show in app".into_button().on_click({
                    let app = app.clone();
                    move |_| {
                        example_message().open(&app);
                    }
                }))
                .into_rows()
                .centered()
                .expand()
                .and(modal)
                .into_layers(),
        )
        .open(app)?;
    Ok(())
}

fn example_message() -> MessageBox {
    MessageBox::message("This is a dialog").with_explanation("This is some explanation text")
}
