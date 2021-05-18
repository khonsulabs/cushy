/// The rendering style of the font.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum FontStyle {
    /// The regular font variant. This is the default if not specified.
    Regular,
    /// An italic font variant.
    Italic,
    /// An oblique font variant.
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Regular
    }
}

impl stylecs::UnscaledStyleComponent for FontStyle {}
