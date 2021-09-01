use std::{fmt::Debug, sync::Arc};

use figures::{Point, Size};

use crate::Pixels;

/// Represents a window.
pub trait Window: Debug + Send + Sync + 'static {
    /// Sets the window title.
    fn set_title(&self, title: &str);

    /// Returns the size of the content area of the window.
    fn inner_size(&self) -> Size<u32, Pixels>;
    /// Attempts to resize the window to `new_size`. This may not work on all platforms.
    fn set_inner_size(&self, new_size: Size<u32, Pixels>);
    /// Returns the position on the screen of the window's top-left corner. On
    /// platforms where this is unsupported, `innner_position()` is returned.
    fn outer_position(&self) -> Point<i32, Pixels> {
        self.inner_position()
    }

    /// Sets the outer position of the window. This may not work on all platforms.
    fn set_outer_position(&self, new_position: Point<i32, Pixels>);

    /// Returns the position of the top-left of the content area in screen coordinates.
    fn inner_position(&self) -> Point<i32, Pixels>;

    /// Sets whether the window should always be on top of other windows.
    fn set_always_on_top(&self, always: bool);

    /// Returns true if the window is maximized.
    fn maximized(&self) -> bool;
    /// Sets whether the window should be maximized.
    fn set_maximized(&self, maximized: bool);
    /// Sets whether the window should be minimized.
    fn set_minimized(&self, minimized: bool);

    /// Closes the window.
    fn close(&self);
}

/// A clonable reference to a window.
pub type WindowRef = Arc<dyn Window>;
