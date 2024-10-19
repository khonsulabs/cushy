use cushy::figures::units::Lp;
use cushy::kludgine::cosmic_text::FamilyOwned;
use cushy::styles::components::FontFamily;
use cushy::styles::{Edges, FontFamilyList};
use cushy::widget::MakeWidget;
use cushy::Run;

fn main() -> cushy::Result {
    include_str!("./nested-scroll.rs")
        .vertical_scroll()
        .with(&FontFamily, FontFamilyList::from(FamilyOwned::Monospace))
        .height(Lp::inches(3))
        .and(
            include_str!("./canvas.rs")
                .vertical_scroll()
                .with(&FontFamily, FontFamilyList::from(FamilyOwned::Monospace))
                .height(Lp::inches(3)),
        )
        .into_rows()
        .pad_by(Edges::default().with_right(Lp::points(7)))
        .vertical_scroll()
        .expand()
        .run()
}
