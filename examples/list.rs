use cushy::value::Dynamic;
use cushy::widget::{MakeWidget, WidgetList};
use cushy::widgets::list::ListStyle;
use cushy::Run;

fn main() -> cushy::Result {
    let current_style: Dynamic<ListStyle> = Dynamic::default();
    let options = ListStyle::provided()
        .into_iter()
        .map(|style| current_style.new_radio(style.clone(), format!("{style:?}")))
        .collect::<WidgetList>();

    let rows = (1..100).map(|i| i.to_string()).collect::<WidgetList>();

    options
        .into_rows()
        .vertical_scroll()
        .and(
            rows.into_list()
                .style(current_style)
                .vertical_scroll()
                .expand(),
        )
        .into_columns()
        .expand()
        .pad()
        .run()
}
