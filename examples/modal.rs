use cushy::dialog::ShouldClose;
use cushy::reactive::value::{Dynamic, Source, Validations};
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
            let validations = form.validations();
            new_item_dialog
                .build_dialog(&form)
                .with_default_button("Ok", {
                    let validations = validations.clone();
                    let new_item_dialog = new_item_dialog.clone();
                    move || {
                        match validations.is_valid() {
                            false => ShouldClose::Remain,
                            true => {
                                // show an 'Are you sure? (Yes/No)' modal
                                new_item_dialog
                                    .build_nested_dialog("Are you sure")
                                    .with_button("Yes", {
                                        let new_item_dialog = new_item_dialog.clone();
                                        let form = form.clone();
                                        move || {
                                            // The values from the from can be accessed here.
                                            println!(
                                                "Ok, and user was certain. Name: {}, Kind: {:?}",
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
                        }
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

    button_container
        .centered()
        .and(modal)
        .into_layers()
        .expand()
        .run()
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum Kind {
    #[default]
    None,
    A,
    B,
}

#[derive(Default, Clone)]
pub struct ItemForm {
    pub name: Dynamic<String>,
    pub kind: Dynamic<Kind>,

    validations: Validations,
}

impl ItemForm {
    pub fn validations(&self) -> &Validations {
        &self.validations
    }
}

impl MakeWidget for &ItemForm {
    fn make_widget(self) -> cushy::widget::WidgetInstance {
        let name_input = Input::new(self.name.clone())
            .placeholder("Enter a name")
            .validation(
                self.validations
                    .validate(&self.name, ItemForm::validate_name),
            )
            .hint("* required");

        let name_form_item = "Name".into_label().and(name_input).into_rows();

        let kind_choices = self
            .kind
            .new_radio(Kind::A)
            .labelled_by("A")
            .and(self.kind.new_radio(Kind::B).labelled_by("B"))
            .into_columns()
            .validation(
                self.validations
                    .validate(&self.kind, ItemForm::validate_kind),
            )
            .hint("* required");

        let kind_form_item = "Kind".into_label().and(kind_choices).into_rows();

        name_form_item.and(kind_form_item).into_rows().make_widget()
    }
}

impl ItemForm {
    #[allow(clippy::ptr_arg)]
    fn validate_name(input: &String) -> Result<(), &'static str> {
        if input.is_empty() {
            Err("This field cannot be empty")
        } else {
            Ok(())
        }
    }

    fn validate_kind(kind: &Kind) -> Result<(), &'static str> {
        match kind {
            Kind::None => Err("Choose an option"),
            _ => Ok(()),
        }
    }
}
