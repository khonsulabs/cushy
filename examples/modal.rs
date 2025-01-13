use cushy::dialog::ShouldClose;
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::label::Displayable;
use cushy::widgets::layers::Modal;
use cushy::widgets::Input;
use cushy::Run;

fn main() -> cushy::Result {
    let modal = Modal::new();

    // simple message example
    let button_1 = "Show Modal Message"
        .into_button()
        .on_click({
            let modal = modal.clone();
            move |_| modal.message("This is a modal", "Dismiss")
        })
        .align_top();

    // form modal, with nested confirmation modal
    let button_2 = "Show Modal Form".into_button().on_click({
        let new_item_dialog = modal.new_handle();
        move |_| {
            let form = ItemForm::default();
            new_item_dialog
                .build_dialog(&form)
                .with_default_button("Ok", {
                    // show an 'Are you sure? (Yes/No)' modal
                    let new_item_dialog = new_item_dialog.clone();
                    move || {
                        new_item_dialog
                            .build_nested_dialog("Are you sure")
                            .with_button("Yes", {
                                let new_item_dialog = new_item_dialog.clone();
                                let form = form.clone();
                                move || {
                                    println!(
                                        "Ok, and user was certain. Insert {}/{:?}",
                                        form.name.get(),
                                        form.kind.get()
                                    );

                                    new_item_dialog.dismiss();
                                    ShouldClose::Close
                                }
                            })
                            .with_default_button("No", || {
                                println!("Ok, and user was unsure");
                                ShouldClose::Close
                            })
                            .show();
                        ShouldClose::Remain
                    }
                })
                .with_cancel_button("Cancel", {
                    || {
                        println!("Cancelled");
                        ShouldClose::Close
                    }
                })
                .show()
        }
    });

    let button_container = button_1.and(button_2).into_columns().contain();

    button_container.and(modal).into_layers().centered().run()
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Kind {
    #[default]
    None,
    A,
    B,
}

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct ItemForm {
    pub name: Dynamic<String>,
    pub kind: Dynamic<Kind>,
}

impl MakeWidget for &ItemForm {
    fn make_widget(self) -> cushy::widget::WidgetInstance {
        let name_input = Input::new(self.name.clone()).placeholder("Enter a name");

        let name_form_item = "Name".into_label().and(name_input).into_rows();

        let kind_choices = self
            .kind
            .new_radio(Kind::A)
            .labelled_by("A")
            .and(self.kind.new_radio(Kind::B).labelled_by("B"))
            .into_columns();

        let kind_form_item = "Kind".into_label().and(kind_choices).into_rows();

        name_form_item.and(kind_form_item).into_rows().make_widget()
    }
}
