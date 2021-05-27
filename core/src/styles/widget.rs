use stylecs::Style;

/// The styles applied to a widget in its various states.
#[derive(Default, Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct WidgetStyle {
    /// The normal styling for the widget.
    pub normal: Style,
    /// The style of the widget when hovered (cursor or screen reader is above).
    pub hover: Style,
    /// The style of the widget when focused.
    pub focus: Style,
    /// The style of the widget while activated (for example, a button is active
    /// while being pressed).
    pub active: Style,
}

impl From<Style> for WidgetStyle {
    fn from(style: Style) -> Self {
        Self {
            normal: style.clone(),
            active: style.clone(),
            hover: style.clone(),
            focus: style,
        }
    }
}

impl WidgetStyle {
    /// Merges `self` with `other`, returning a new instance with the results.
    /// Uses [`Style::merge_with`] for each of the states.
    #[must_use]
    pub fn merge_with(&self, other: &Self, is_inheritance: bool) -> Self {
        Self {
            normal: self.normal.merge_with(&other.normal, is_inheritance),
            active: self.active.merge_with(&other.active, is_inheritance),
            hover: self.hover.merge_with(&other.hover, is_inheritance),
            focus: self.focus.merge_with(&other.focus, is_inheritance),
        }
    }

    // /// Create a new WidgetStyle
    // #[must_use]
    // pub fn map_each<F: Fn(&Style) -> Style>(&self, map: F) -> Self {
    //     Self {
    //         normal: map(&self.normal),
    //         active: map(&self.active),
    //         hover: map(&self.hover),
    //         focus: map(&self.focus),
    //     }
    // }
}
