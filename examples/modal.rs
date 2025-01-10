use cushy::widget::MakeWidget;
use cushy::widgets::layers::Modal;
use cushy::Run;
use cushy::value::Dynamic;
use cushy::widgets::Input;
use cushy::widgets::label::Displayable;

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
    let button_2 = "Show Modal Form"
        .into_button()
        .on_click({
            let modal = modal.clone();
            move |_| {
                modal
                    .build_dialog(make_form())
                    .with_default_button("Ok", {
                        // show an 'Are you sure? (Yes/No)' modal
                        // FIXME what we want is for the form to still be visible, but the form modal
                        //       has already been closed at this point
                        let modal = modal.clone();
                        move || modal.build_dialog("Are you sure")
                            .with_button("Yes", || {
                                println!("Ok, and user was certain");

                                // TODO show how to get/use the name/kind values from the dialog here
                            })
                            .with_default_button("No", || {
                                println!("Ok, and user was unsure");
                            })
                            .show()
                    })
                    .with_cancel_button("Cancel", {
                        || {
                            println!("Cancelled");
                        }
                    })

                    .show()
            }
        });

    let button_container = button_1
        .and(button_2)
        .into_columns()
        .contain();

    button_container
        .and(modal)
        .into_layers()
        .centered()
        .run()
}


#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Kind {
    #[default]
    None,
    A,
    B,
}
pub fn make_form() -> impl MakeWidget {
    let name: Dynamic<String> = Dynamic::default();

    let kind: Dynamic<Kind> = Dynamic::default();

    let name_input = Input::new(name)
        .placeholder("Enter a name");

    let name_form_item = "Name"
        .into_label()
        .and(name_input)
        .into_rows();


    let kind_choices = kind.new_radio(Kind::A)
        .labelled_by("A")
        .and(kind.new_radio(Kind::B).labelled_by("B"))
        .into_columns();


    let kind_form_item = "Kind"
        .into_label()
        .and(kind_choices)
        .into_rows();

    let form = name_form_item
        .and(kind_form_item)
        .into_rows();

    form
        .make_widget()
}
