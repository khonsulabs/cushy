use stylecs::StyleComponent;

/// Indicates to `Gooey` that it should attempt to focus this Widget upon
/// initialization.
#[derive(Debug, Clone, Copy)]
pub struct Autofocus;

impl StyleComponent for Autofocus {
    fn should_be_inherited(&self) -> bool {
        false
    }
}

/// Indicates the index in the tab order for the window. The indicies must be
/// consistent within the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TabIndex(pub usize);

impl StyleComponent for TabIndex {
    fn should_be_inherited(&self) -> bool {
        false
    }
}
