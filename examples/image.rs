use cushy::animation::ZeroToOne;
use cushy::value::{Dynamic, MapEachCloned, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::image::{Aspect, ImageScaling};
use cushy::widgets::slider::Slidable;
use cushy::widgets::Image;
use cushy::Run;
use figures::Size;
use kludgine::include_texture;

fn main() -> cushy::Result {
    let mode = Dynamic::<ScalingMode>::default();
    let scale = Dynamic::new(1f32);
    let orientation_width = Dynamic::<ZeroToOne>::default();
    let orientation_height = Dynamic::<ZeroToOne>::default();
    let orientation = (&orientation_width, &orientation_height)
        .map_each_cloned(|(width, height)| Size::new(width, height));
    let aspect_mode = Dynamic::<Aspect>::default();
    let image_scaling = (&mode, &scale, &aspect_mode, &orientation).map_each_cloned(
        |(mode, scale, aspect_mode, orientation)| match mode {
            ScalingMode::Aspect => ImageScaling::Aspect {
                mode: aspect_mode,
                orientation,
            },
            ScalingMode::Stretch => ImageScaling::Stretch,
            ScalingMode::Scale => ImageScaling::Scale(scale),
        },
    );
    let hide_scale_editor = mode.map_each(|scale| !matches!(scale, ScalingMode::Scale));
    let scale_editor = scale
        .slider_between(0., 3.)
        .contain()
        .collapse_vertically(hide_scale_editor);

    let image = Image::new(include_texture!("assets/ferris-happy.png").expect("valid image"));

    let origin_select = "Origin"
        .h3()
        .and("Width Orientation")
        .and(orientation_width.slider())
        .and("Height Orientation")
        .and(orientation_height.slider())
        .into_rows();

    let apsect_mode_select = "Mode"
        .h3()
        .and(aspect_mode.new_radio(Aspect::Fit, "Fit"))
        .and(aspect_mode.new_radio(Aspect::Fill, "Fill"))
        .into_rows();

    let hide_aspect_editor = mode.map_each(|scale| !matches!(scale, ScalingMode::Aspect));
    let aspect_editor = origin_select
        .and(apsect_mode_select)
        .into_rows()
        .contain()
        .collapse_vertically(hide_aspect_editor);

    let mode_select = "Scaling Mode"
        .h1()
        .and(mode.new_radio(ScalingMode::Scale, "Scale"))
        .and(scale_editor)
        .and(
            mode.new_radio(ScalingMode::Aspect, "Aspect")
                .and(aspect_editor)
                .into_rows(),
        )
        .and(mode.new_radio(ScalingMode::Stretch, "Stretch"))
        .into_rows();

    mode_select
        .and(image.scaling(image_scaling).expand().contain())
        .into_columns()
        .pad()
        .expand()
        .run()
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
enum ScalingMode {
    Aspect,
    Stretch,
    #[default]
    Scale,
}
