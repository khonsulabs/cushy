use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::widgets::Canvas;
use cushy::Run;
use plotters::prelude::*;

// This is copied from the sierpinski.rs example in the plotters repository.
// This just demonstrates that any `plotters` code that renders to a
// `DrawingArea` can be used with a `Canvas`.
pub fn sierpinski_carpet<A>(
    depth: u32,
    drawing_area: &DrawingArea<A, plotters::coord::Shift>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: DrawingBackend,
    A::ErrorType: 'static,
{
    if depth > 0 {
        let sub_areas = drawing_area.split_evenly((3, 3));
        for (idx, sub_area) in (0..).zip(sub_areas.iter()) {
            if idx != 4 {
                sub_area.fill(&BLUE)?;
                sierpinski_carpet(depth - 1, sub_area)?;
            } else {
                sub_area.fill(&WHITE)?;
            }
        }
    }
    Ok(())
}

fn main() -> cushy::Result<()> {
    let depth = Dynamic::new(1);

    "Depth"
        .and(depth.clone().slider_between(1, 5))
        .and(
            Canvas::new({
                move |context| {
                    let depth = depth.get_tracking_redraw(context);
                    sierpinski_carpet(depth, &context.gfx.as_plot_area()).unwrap();
                }
            })
            .expand(),
        )
        .into_rows()
        .run()
}
