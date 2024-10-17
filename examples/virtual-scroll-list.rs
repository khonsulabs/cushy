use cushy::styles::{Dimension, DimensionRange, Edges};
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::Run;
use figures::units::{Lp, UPx};
use figures::{Point, Size};

fn list() -> impl MakeWidget {
    let height = Lp::inches(10);
    let content_size: Dynamic<Size<UPx>> = Dynamic::default();
    let control_size = Dynamic::default();
    let current_scroll: Dynamic<Point<UPx>> = Dynamic::default();
    let max_scroll = Dynamic::default();

    let content = content_size.map_each(|s| format!("Content size: {:?};", s));
    let control = control_size.map_each(|s| format!("Control size: {:?};", s));
    let scroll = current_scroll.map_each(|s| format!("Current scroll: {:?};", s));
    let max = max_scroll.map_each(|s| format!("Max scroll: {:?};", s));

    let content = content
        .and(control)
        .and(scroll)
        .and(max)
        .into_columns()
        .and("Hello world!")
        .into_rows()
        .pad_by(current_scroll.map_each(|scroll| Edges {
            top: Dimension::from(scroll.y),
            ..Default::default()
        }))
        .size(Size::new(
            DimensionRange::default(),
            DimensionRange::from(height),
        ));

    let scroll = content.scroll();

    scroll
        .content_size()
        .for_each_cloned(move |s| content_size.set(s))
        .persist();
    scroll
        .control_size()
        .for_each_cloned(move |s| control_size.set(s))
        .persist();
    scroll
        .scroll
        .for_each_cloned(move |s| current_scroll.set(s))
        .persist();
    scroll
        .max_scroll()
        .for_each_cloned(move |s| max_scroll.set(s))
        .persist();

    scroll.expand()
}

fn main() -> cushy::Result {
    list().run()
}

#[test]
fn runs() {
    cushy::example!(list).untested_still_frame();
}
