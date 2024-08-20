use cushy::figures::units::Px;
use cushy::fonts::FontCollection;
use cushy::styles::components::{FontFamily, FontWeight, LineHeight, TextSize};
use cushy::styles::{Component, DynamicComponent, FamilyOwned, FontFamilyList};
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::widgets::input::InputValue;
use cushy::Run;

fn main() -> cushy::Result<()> {
    let file_path = Dynamic::<String>::default();
    let fonts = FontCollection::default();
    let font_data = file_path.map_each(|path| std::fs::read(path).map_err(|err| err.to_string()));
    let loaded_font = font_data.map_each({
        let fonts = fonts.clone();
        move |result| {
            result
                .as_ref()
                .ok()
                .map(|data| fonts.push_unloadable(data.to_vec()))
        }
    });
    let primary_family_name = DynamicComponent::new({
        let loaded_font = loaded_font.clone();
        move |context| {
            let font = loaded_font.get_tracking_invalidate(context)?;

            let face = context.loaded_font_faces(&font).first()?;
            Some(Component::custom(FontFamilyList::from(vec![
                FamilyOwned::Name(face.families[0].0.clone()),
            ])))
        }
    });
    let family_weight = DynamicComponent::new(move |context| {
        let font = loaded_font.get_tracking_invalidate(context)?;

        let face = context.loaded_font_faces(&font).first()?;

        Some(Component::FontWeight(face.weight))
    });

    let mut window = file_path
        .into_input()
        .validation(font_data.clone())
        .and(
            "The quick brown fox jumps over the lazy dog."
                .with(&TextSize, Px::new(36))
                .with(&LineHeight, Px::new(36))
                .with_dynamic(&FontFamily, primary_family_name)
                .with_dynamic(&FontWeight, family_weight),
        )
        .into_rows()
        .into_window();
    window.fonts = fonts;
    window.run()
}
