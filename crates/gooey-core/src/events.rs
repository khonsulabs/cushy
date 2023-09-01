use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Debug, Not, TryFrom};
use figures::units::Px;
use figures::{Point, Rect};

pub struct KeyEvent {
    pub scan_code: u32,
    pub virtual_keycode: Option<VirtualKeyCode>,
    // TODO should we include the localized key as well? i.e. z -> y and [ -> ü on a german keyboard
    // TODO modifiers
}

// TODO this is winit's enum, but maybe we should do some research what would be best also with
// respect to web
/// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, TryFrom)]
#[repr(u32)]
pub enum VirtualKeyCode {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    Backspace,
    /// The Enter key.
    Return,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    // also called "Next"
    NavigateForward,
    // also called "Prior"
    NavigateBackward,
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}

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
