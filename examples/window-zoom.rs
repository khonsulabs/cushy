use cushy::figures::Fraction;
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::Run;

fn main() -> cushy::Result<()> {
    let zoom = Dynamic::new(Fraction::ONE);
    zoom.map_each(|z| z.to_string())
        .and(
            zoom.clone()
                .slider_between(Fraction::new(1, 4), Fraction::new(4, 1)),
        )
        .into_rows()
        .fit_horizontally()
        .pad()
        .expand()
        .into_window()
        .zoom(zoom)
        .run()
}
