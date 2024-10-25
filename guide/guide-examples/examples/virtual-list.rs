use cushy::figures::units::Lp;
use cushy::styles::Dimension;
use cushy::widget::MakeWidget;
use cushy::widgets::virtual_list::{VirtualList, VirtualListContent};
use cushy::Run;

// ANCHOR: implementation
#[derive(Debug)]
struct TestVirtualList;

impl VirtualListContent for TestVirtualList {
    fn item_count(&self) -> impl cushy::value::IntoValue<usize> {
        50
    }
    fn item_height(&self) -> impl cushy::value::IntoValue<cushy::styles::Dimension> {
        cushy::styles::Dimension::Lp(Lp::inches_f(0.5))
    }
    fn widget_at(&self, index: usize) -> impl MakeWidget {
        format!("Item {}", index)
    }
    fn width(&self) -> impl cushy::value::IntoValue<cushy::styles::Dimension> {
        Dimension::Lp(Lp::inches_f(10.))
    }
}
// ANCHOR_END: implementation

// ANCHOR: list
fn list() -> impl MakeWidget {
    VirtualList::new(TestVirtualList).expand()
}
// ANCHOR_END: list

fn main() -> cushy::Result {
    list().run()
}

#[test]
fn runs() {
    cushy::example!(list).untested_still_frame();
}
