use cushy::figures::units::Lp;
use cushy::value::{Dynamic, IntoReader};
use cushy::widget::MakeWidget;
use cushy::Run;

fn main() -> cushy::Result {
    let counter = Dynamic::new(0i32);

    counter
        .to_label()
        .width(Lp::points(100))
        .and("+".into_button().on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() += 1;
            }
        })))
        .and("-".into_button().on_click(counter.with_clone(|counter| {
            move |_| {
                *counter.lock() -= 1;
            }
        })))
        .into_columns()
        .centered()
        .run()
}
