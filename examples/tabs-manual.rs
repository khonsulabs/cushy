//! This example show show to use a stack of buttons and a switcher to achieve a
//! tab-like widget.

use std::collections::HashMap;

use cushy::reactive::value::{Dynamic, Switchable};
use cushy::widget::MakeWidget;
use cushy::Run;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum Tab {
    First,
    Second,
    Missing,
}

fn main() -> cushy::Result {
    let mut tab_contents = HashMap::new();
    tab_contents.insert(Tab::First, "This is the first tab!".make_widget());
    tab_contents.insert(Tab::Second, "This is the second tab!".make_widget());

    let selected_tab = Dynamic::new(Tab::First);

    let tabs = selected_tab
        .new_select(Tab::First, "First")
        .and(selected_tab.new_select(Tab::Second, "Second"))
        .and(selected_tab.new_select(Tab::Missing, "Missing"))
        .into_columns();

    tabs.and(selected_tab.switch_between(tab_contents))
        .into_rows()
        .fit_vertically()
        .run()
}
