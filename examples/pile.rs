use cushy::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetList};
use cushy::widgets::input::InputValue;
use cushy::widgets::pile::Pile;
use cushy::Run;

fn main() -> cushy::Result {
    let pile = Pile::default();
    let mut counter = 0;
    let buttons = Dynamic::<WidgetList>::default();
    buttons.lock().push("+".into_button().on_click({
        let buttons = buttons.clone();
        let pile = pile.clone();
        move |_| {
            counter += 1;

            let pending_section = pile.new_pending();
            let handle = pending_section.clone();
            let button = format!("{counter}")
                .into_button()
                .on_click({
                    let section = handle.clone();
                    move |_| section.show_and_focus()
                })
                .make_widget();
            let button_id = button.id();

            pending_section.finish(
                Dynamic::new(format!("Section {counter}"))
                    .into_input()
                    .and("Close Section".into_button().on_click({
                        let buttons = buttons.clone();
                        move |_| {
                            // Remove the section widget.
                            handle.remove();
                            // Remove the button.
                            buttons.lock().retain(|button| button.id() != button_id);
                        }
                    }))
                    .into_rows()
                    .centered(),
            );
            let mut buttons = buttons.lock();
            let index = buttons.len() - 1;
            buttons.insert(index, button)
        }
    }));

    buttons
        .into_columns()
        .and(pile.centered().expand())
        .into_rows()
        .expand()
        .run()
}
