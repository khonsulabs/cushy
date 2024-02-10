use cushy::styles::components::FontFamily;
use cushy::styles::FontFamilyList;
use cushy::widget::MakeWidget;
use cushy::Run;
use figures::units::Lp;
use kludgine::cosmic_text::FamilyOwned;

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
        .vertical_scroll()
        .expand()
        .run()
}
