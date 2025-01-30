use cushy::reactive::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::label::Displayable;
use cushy::Run;
use figures::units::Lp;

fn main() -> cushy::Result {
    let count = Dynamic::new(0_usize);

    count
        .to_label()
        .expand()
        .and(
            "Count"
                .into_button()
                .on_click(move |_| {
                    *count.lock() += 1;
                })
                .expand(),
        )
        .into_columns()
        .pad()
        .width(Lp::inches(3))
        .into_window()
        .titled("Counter")
        .resize_to_fit(true)
        .run()
}
