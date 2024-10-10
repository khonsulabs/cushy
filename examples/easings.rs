use cushy::animation::easings::StandardEasing;
use cushy::context::GraphicsContext;
use cushy::kludgine::shapes::{PathBuilder, Shape, StrokeOptions};
use cushy::widget::{MakeWidget, WidgetList};
use cushy::widgets::Canvas;
use cushy::Run;
use easing_function::Easing;
use figures::units::{Lp, Px};
use figures::{IntoSigned, Point, Rect, Size, Zero};

fn main() -> cushy::Result {
    StandardEasing::all()
        .iter()
        .map(|easing| {
            let name = format!("Ease{easing:?}");

            Canvas::new(|context| {
                draw_easing_graph(easing, context);
            })
            .expand()
            .and(name)
            .into_rows()
            .contain()
            .make_widget()
        })
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|widgets| {
            WidgetList::from_iter(widgets.iter().map(|w| w.clone().expand()))
                .into_columns()
                .height(Lp::inches(3))
        })
        .collect::<WidgetList>()
        .into_wrap()
        .pad()
        .vertical_scroll()
        .expand()
        .run()
}

fn draw_easing_graph(easing: &StandardEasing, context: &mut GraphicsContext<'_, '_, '_, '_>) {
    let height = context.gfx.size().height.into_signed();
    let padding = height / 4;
    let height = height - padding * 2;
    let width = context.gfx.size().width.into_signed().get();
    let steps = width.max(50);
    let mut path = PathBuilder::new(Point::new(
        Px::ZERO,
        padding + height * (1.0 - easing.ease(0.)),
    ));

    for i in 1..=steps {
        path = path.line_to(Point::new(
            Px::new(width * i) / steps,
            padding + height * (1.0 - easing.ease(i as f32 / steps as f32)),
        ));
    }

    let text_color = context.theme().surface.on_color;
    let bg = context.theme().surface.low_container;
    let outline = context.theme().surface.outline;
    context.gfx.draw_shape(&Shape::filled_rect(
        Rect::new(
            Point::new(Px::ZERO, padding),
            Size::new(Px::new(width), height),
        ),
        bg,
    ));
    context.gfx.draw_shape(&Shape::stroked_rect(
        Rect::new(
            Point::new(Px::ZERO, padding),
            Size::new(Px::new(width), height),
        ),
        outline,
    ));

    context.gfx.draw_shape(
        &path
            .build()
            .stroke(StrokeOptions::px_wide(1).colored(text_color)),
    );
}
