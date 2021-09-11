use stylecs::StyleComponent;

/// An intent for a widget. This is used to track which widget should process
/// "default" and "cancel" intents that come from the keyboard or accessible
/// devices.
///
/// This shouldn't be attached to a widget directly, as the specific frontend
/// may not be able to support arbitrary widgets with this intent. Instead, look
/// for a widget such as `gooey-widgets::Button` that allows configuring itself
/// to have a specific intent.
///
/// If you're looking to implement a custom widget that supports these intents,
/// look to the `Button` widget as an example for the frontends that `Gooey`
/// supports.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Intent {
    /// Indicates this widget should be interacted with when a user wishes to
    /// the default action. This is usually in response to pressing the enter or
    /// return key.
    Default,
    /// Indicates this widget should be interacted with when a user wishes to
    /// cancel the current operation. This is usually in response to pressing
    /// the escape key.
    Cancel,
}

impl StyleComponent for Intent {
    fn should_be_inherited(&self) -> bool {
        false
    }
}
