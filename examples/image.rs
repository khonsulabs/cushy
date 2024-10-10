use std::collections::HashMap;

use cushy::animation::ZeroToOne;
use cushy::figures::Size;
use cushy::kludgine::include_texture;
use cushy::kludgine::wgpu::FilterMode;
use cushy::value::{Dynamic, MapEachCloned, Source, Switchable};
use cushy::widget::MakeWidget;
use cushy::widgets::image::{Aspect, ImageScaling};
use cushy::widgets::slider::Slidable;
use cushy::widgets::Image;
use cushy::Run;

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

    let image_nearest = Image::new(
        include_texture!("assets/ferris-happy.png", FilterMode::Nearest).expect("valid image"),
    );
    let image_linear = Image::new(
        include_texture!("assets/ferris-happy.png", FilterMode::Linear).expect("valid image"),
    );
    let mut images = HashMap::new();

    images.insert(
        FilterMode::Nearest,
        image_nearest.scaling(image_scaling.clone()).make_widget(),
    );
    images.insert(
        FilterMode::Linear,
        image_linear.scaling(image_scaling).make_widget(),
    );

    let selected_filter = Dynamic::new(FilterMode::Nearest);

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

    let filter_select = "Filter mode"
        .h1()
        .and(selected_filter.new_radio(FilterMode::Nearest, "Nearest"))
        .and(selected_filter.new_radio(FilterMode::Linear, "Linear"))
        .into_rows();

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
        .and(filter_select)
        .into_rows();

    mode_select
        .and(selected_filter.switch_between(images).expand().contain())
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
