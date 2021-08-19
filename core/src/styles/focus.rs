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

/// Indicates the order in which focus moves between Widgets on a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TabOrder(usize);

impl StyleComponent for TabOrder {
    fn should_be_inherited(&self) -> bool {
        false
    }
}
