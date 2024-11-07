use cushy::widget::MakeWidget;
use cushy::widgets::VirtualList;
use cushy::Run;

fn list() -> impl MakeWidget {
    VirtualList::new(50, |index| format!("Item {}", index)).expand()
}

fn main() -> cushy::Result {
    list().run()
}

#[test]
fn runs() {
    cushy::example!(list).untested_still_frame();
}
