use std::panic::UnwindSafe;

use figures::units::{Px, UPx};
use figures::{Point, Size};
use gooey_reactor::Dynamic;

use crate::Context;

#[derive(Default)]
#[must_use]
pub struct WindowBuilder {
    attributes: WindowAttributes,
}

impl WindowBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.attributes.title = title.into();
        self
    }

    pub fn level(mut self, level: WindowLevel) -> Self {
        self.attributes.window_level = level;
        self
    }

    pub fn location(mut self, location: Point<Px>) -> Self {
        self.attributes.location = Some(location);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.attributes.resizable = resizable;
        self
    }

    pub fn inner_size(mut self, size: Size<UPx>) -> Self {
        self.attributes.inner_size = Some(size);
        self
    }

    pub fn create<Widget, Initializer>(self, init: Initializer) -> NewWindow<Widget>
    where
        Initializer: FnOnce(&Context, &Window) -> Widget + Send + UnwindSafe + 'static,
    {
        NewWindow {
            attributes: self.attributes,
            init: Box::new(init),
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
pub struct WindowAttributes {
    pub inner_size: Option<Size<UPx>>,
    pub min_inner_size: Option<Size<UPx>>,
    pub max_inner_size: Option<Size<UPx>>,
    pub location: Option<Point<Px>>,
    pub resizable: bool,
    pub enabled_buttons: WindowButtons,
    pub title: String,
    // pub fullscreen: Option<Fullscreen>,
    pub maximized: bool,
    pub visible: bool,
    pub transparent: bool,
    pub decorations: bool,
    // pub window_icon: Option<Icon>,
    // pub preferred_theme: Option<Theme>,
    pub resize_increments: Option<Size<UPx>>,
    pub content_protected: bool,
    pub window_level: WindowLevel,
    // pub parent_window: Option<Window<ParentWindowEvent>>,
    pub active: bool,
}

impl Default for WindowAttributes {
    fn default() -> Self {
        WindowAttributes {
            inner_size: None,
            min_inner_size: None,
            max_inner_size: None,
            location: None,
            resizable: true,
            enabled_buttons: WindowButtons::all(),
            title: "Gooey".to_owned(),
            maximized: false,
            // fullscreen: None,
            visible: true,
            transparent: false,
            decorations: true,
            window_level: WindowLevel::default(),
            // window_icon: None,
            // preferred_theme: None,
            resize_increments: None,
            content_protected: false,
            // parent_window: None,
            active: true,
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum WindowLevel {
    /// The window will always be below normal windows.
    ///
    /// This is useful for a widget-based app.
    AlwaysOnBottom,
    /// The default.
    #[default]
    Normal,
    /// The window will always be on top of normal windows.
    AlwaysOnTop,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct WindowButtons(u8);

impl WindowButtons {
    pub const CLOSE: Self = Self(1 << 0);
    pub const MAXIMIZE: Self = Self(1 << 2);
    pub const MINIMIZE: Self = Self(1 << 1);

    #[must_use]
    pub const fn all() -> Self {
        Self(Self::CLOSE.0 | Self::MAXIMIZE.0 | Self::MINIMIZE.0)
    }

    #[must_use]
    pub const fn maximize(&self) -> bool {
        self.0 & Self::MAXIMIZE.0 != 0
    }

    #[must_use]
    pub const fn minimize(&self) -> bool {
        self.0 & Self::MINIMIZE.0 != 0
    }

    #[must_use]
    pub const fn close(&self) -> bool {
        self.0 & Self::CLOSE.0 != 0
    }
}

pub type WindowInitializer<Widget> =
    Box<dyn FnOnce(&crate::Context, &Window) -> Widget + std::panic::UnwindSafe + Send>;

pub struct NewWindow<Widget> {
    pub attributes: WindowAttributes,
    pub init: WindowInitializer<Widget>,
}

pub struct Window {
    pub inner_size: Dynamic<Size<UPx>>,
    pub location: Dynamic<Point<Px>>,
    pub title: Dynamic<String>,
}

impl Window {
    #[must_use]
    pub fn new(attrs: WindowAttributes, cx: &Context) -> Self {
        Self {
            inner_size: cx.new_dynamic(attrs.inner_size.unwrap_or_default()),
            location: cx.new_dynamic(attrs.location.unwrap_or_default()),
            title: cx.new_dynamic(attrs.title),
        }
    }
}
