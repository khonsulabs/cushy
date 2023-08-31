use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Debug, Not};
use figures::units::Px;
use figures::{Point, Rect};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MouseEvent {
    /// All currently pressed buttons
    pub current_buttons: MouseButtons,
    /// Button that triggered the event
    ///
    /// - for `pressed` this button is also contained in `current_buttons`
    /// - for `released` this button is not contained in `current_buttons`
    /// - for `moved` this is equivalent to `current_buttons`, i.e., all pressed buttons
    pub button: MouseButtons,
    pub position: Option<Point<Px>>,
}

impl MouseEvent {
    #[must_use]
    pub fn with_position(mut self, position: Option<Point<Px>>) -> Self {
        self.position = position;
        self
    }
}

impl MouseEvent {
    /// Returns relative mouse position if contained in parent rect.
    #[must_use]
    pub fn relative(&self, parent: Rect<Px>) -> Option<Point<Px>> {
        let relative = self.position.map(|pos| pos - parent.origin);
        if relative.map_or(false, |relative| {
            relative.x >= 0
                && relative.y >= 0
                && relative.x < parent.size.width
                && relative.y < parent.size.height
        }) {
            relative
        } else {
            None
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, BitAnd, BitOr, BitAndAssign, BitOrAssign, Not,
)]
#[must_use]
pub struct MouseButtons(#[debug("{_0:b}")] u64);

#[rustfmt::skip]
impl MouseButtons {
    /// Primary button.
    pub const LEFT: Self = MouseButtons::single(0);
    /// Secondary button.
    pub const RIGHT: Self = MouseButtons::single(1);
    /// Auxiliary button (mouse wheel).
    pub const MIDDLE: Self = MouseButtons::single(2);
    /// Also called forth.
    pub const BACK: Self = MouseButtons::single(3);
    /// Also called fifth.
    pub const FORWARD: Self = MouseButtons::single(4);
}

impl MouseButtons {
    /// A `MouseButtons` with the `bitflags` keys set
    ///
    /// - `1` is left
    /// - `2` is middle
    /// - `4` is right
    /// - `8` is back or forth
    /// - `16` is forward or fifth
    ///
    /// For these also exist constants [`Self::LEFT`] - [`Self::FORWARD`]
    /// that can be combined with `|`.
    pub const fn multiple(bitflags: u64) -> Self {
        Self(bitflags)
    }

    /// A `MouseButtons` with the `n`th mouse button set.
    ///
    /// - `0` is left
    /// - `1` is middle
    /// - `2` is right
    /// - `3` is back or forth
    /// - `4` is forward or fifth
    ///
    /// For these also exist constants [`Self::LEFT`] - [`Self::FORWARD`].
    ///
    /// # Panics
    /// Panics when `n >= 64`.
    pub const fn single(n: u8) -> Self {
        assert!(n < 64, "mouse button must be less than 64");
        Self(1 << n)
    }

    /// Sets the `n`th mouse button.
    ///
    /// - `0` is left
    /// - `1` is middle
    /// - `2` is right
    /// - `3` is back or forth
    /// - `4` is forward or fifth
    ///
    /// For these also exist constants [`Self::LEFT`] - [`Self::FORWARD`]
    /// that can be combined with `|`.
    ///
    /// # Panics
    /// Panics when `n >= 64`.
    pub fn with(self, n: u8) -> Self {
        self | Self::single(n)
    }

    /// Any mouse button.
    #[must_use]
    pub fn any(self) -> bool {
        self.0 > 0
    }

    /// The left mouse button.
    #[must_use]
    pub fn left(self) -> bool {
        (self & Self::LEFT).any()
    }

    /// The middle mouse button (wheel).
    #[must_use]
    pub fn middle(self) -> bool {
        (self & Self::MIDDLE).any()
    }

    /// The right mouse button.
    #[must_use]
    pub fn right(self) -> bool {
        (self & Self::RIGHT).any()
    }

    /// The back mouse button (forth).
    ///
    /// Sometimes called forth, zero based it's `n = 3`.
    #[must_use]
    pub fn back(self) -> bool {
        (self & Self::BACK).any()
    }

    /// The forward mouse button (fifth).
    ///
    /// Sometimes called fifth, zero based it's `n = 4`.
    #[must_use]
    pub fn forward(self) -> bool {
        (self & Self::FORWARD).any()
    }

    /// The `n`th mouse button, zero based.
    ///
    /// - `0` is left
    /// - `1` is middle
    /// - `2` is right
    /// - `3` is back or forth
    /// - `4` is forward or fifth
    ///
    /// For these also exist helpers [`Self::left`] - [`Self::forward`].
    ///
    /// # Panics
    /// Panics when `n >= 64`.
    #[must_use]
    pub fn nth(self, n: u8) -> bool {
        (self & Self::single(n)).any()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn construction() {
        let buttons = MouseButtons::single(2).with(3).with(20).with(63);
        assert_eq!(
            buttons,
            MouseButtons::multiple(
                0b1000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001_0000_0000_0000_0000_1100
            )
        );
    }

    #[test]
    fn left() {
        assert!(MouseButtons::LEFT.left());
        assert!(MouseButtons::LEFT.any());
        assert!(!MouseButtons::LEFT.right());
        assert!(MouseButtons::LEFT.nth(0));
        assert!(!MouseButtons::LEFT.nth(1));
        assert_eq!(MouseButtons::LEFT, MouseButtons::single(0));
        assert_eq!(MouseButtons::LEFT, MouseButtons::multiple(1));
    }
}
