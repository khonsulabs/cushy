/// The font family name for drawing text.
#[derive(Debug, Clone)]
pub struct FontFamily(pub String);
impl stylecs::UnscaledStyleComponent for FontFamily {}
impl Default for FontFamily {
    fn default() -> Self {
        Self("Roboto".to_owned())
    }
}

impl<T> From<T> for FontFamily
where
    T: ToString,
{
    fn from(family: T) -> Self {
        Self(family.to_string())
    }
}
