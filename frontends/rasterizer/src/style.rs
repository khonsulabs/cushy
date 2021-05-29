use gooey_core::styles::{
    style_sheet::{Rule, StyleSheet},
    BackgroundColor, ColorPair, Srgba, TextColor,
};

pub const CONTROL_CLASS: &str = "gooey-widgets.control";

pub fn default_stylesheet() -> StyleSheet {
    // Palette from https://flatuicolors.com/palette/defo
    let turqouise = Srgba::new(0.102, 0.737, 0.612, 1.);
    let green_sea = Srgba::new(0.086, 0.627, 0.522, 1.);
    let sunflower = Srgba::new(0.945, 0.769, 0.059, 1.);
    let orange = Srgba::new(0.953, 0.612, 0.071, 1.);
    let emerald = Srgba::new(0.180, 0.800, 0.443, 1.);
    let nephritis = Srgba::new(0.153, 0.682, 0.376, 1.);
    let carrot = Srgba::new(0.902, 0.494, 0.133, 1.);
    let pumpkin = Srgba::new(0.827, 0.329, 0.000, 1.);
    let peter_river = Srgba::new(0.204, 0.596, 0.859, 1.);
    let belize_hole = Srgba::new(0.161, 0.502, 0.725, 1.);
    let alizarin = Srgba::new(0.906, 0.298, 0.235, 1.);
    let pomegranate = Srgba::new(0.753, 0.224, 0.169, 1.);
    let amethyst = Srgba::new(0.608, 0.349, 0.714, 1.);
    let wisteria = Srgba::new(0.557, 0.267, 0.678, 1.);
    let clouds = Srgba::new(0.925, 0.941, 0.945, 1.);
    let silver = Srgba::new(0.741, 0.765, 0.780, 1.);
    let wet_asphalt = Srgba::new(0.204, 0.286, 0.369, 1.);
    let midnight_blue = Srgba::new(0.173, 0.243, 0.314, 1.);
    let asbestos = Srgba::new(0.498, 0.549, 0.553, 1.);
    let concrete = Srgba::new(0.584, 0.647, 0.651, 1.);

    StyleSheet::default()
        .with(Rule::for_classes(CONTROL_CLASS).with_styles(|style| {
            style
                .with(TextColor(ColorPair {
                    light_color: midnight_blue,
                    dark_color: clouds,
                }))
                .with(BackgroundColor(ColorPair {
                    light_color: silver,
                    dark_color: wet_asphalt,
                }))
        }))
        .with(
            Rule::for_classes(CONTROL_CLASS)
                .when_hovered()
                .with_styles(|style| {
                    style.with(BackgroundColor(ColorPair {
                        light_color: concrete,
                        dark_color: asbestos,
                    }))
                }),
        )
}
