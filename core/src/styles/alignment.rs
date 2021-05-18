/// Horizontally aligns items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    /// Horizontally align items to the left.
    Left,
    /// Horizontally align items in the center.
    Center,
    /// Horizontally align items to the right.
    Right,
}
impl stylecs::UnscaledStyleComponent for Alignment {}

impl Default for Alignment {
    fn default() -> Self {
        Self::Left
    }
}

/// Vertically aligns items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
pub enum VerticalAlignment {
    /// Vertically align items to the top.
    Top,
    /// Vertically align items in the center.
    Center,
    /// Vertically align items to the bottom.
    Bottom,
}
impl stylecs::UnscaledStyleComponent for VerticalAlignment {}

impl Default for VerticalAlignment {
    fn default() -> Self {
        Self::Top
    }
}
