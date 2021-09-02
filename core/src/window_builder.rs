use crate::{
    styles::style_sheet::Classes, AnyFrontend, Pixels, StyledWidget, Widget, WidgetRegistration,
    WidgetStorage, ROOT_CLASS,
};
use figures::{Point, Points, Size};
use std::{any::Any, borrow::Cow, fmt::Debug};

type InitializerFn<W> = dyn FnOnce(&WidgetStorage) -> StyledWidget<W>;

/// A builder for a Window.
#[must_use]
pub struct WindowBuilder<W: Widget> {
    /// The function that creates the root widget for this window.
    pub initializer: Option<Box<InitializerFn<W>>>,
    /// The intial configuration of the window.
    pub configuration: WindowConfiguration,
}

/// Configuration options used when opening a window.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct WindowConfiguration {
    /// The title of the window. If not set, "Gooey - Kludgine" will be used.
    pub title: Option<String>,
    /// The initial position of the window. If None, the system will place it by
    /// its default methods. The point is in screen coordinates, relative to the
    /// top-left of the primary display. Coordinates can be negative.
    pub position: Option<Point<i32, Pixels>>,
    /// The initial size of the window. The default value is `Size::new(1024,
    /// 768)`.
    pub size: Size<u32, Points>,
    /// If true, the window can be resized by the user. Defaults to true.
    pub resizable: bool,
    /// If true, the window will start maximized. Defaults to false.
    pub maximized: bool,
    /// If true, where the background color is transparent, the window will show
    /// content behind it. Defaults to false.
    pub transparent: bool,
    /// Determines whether the window should have its normal decorations, such
    /// as the title bar and border.
    pub decorations: bool,
    /// Sets whether the window should always be on top of other windows.
    pub always_on_top: bool,
}

impl Default for WindowConfiguration {
    fn default() -> Self {
        Self {
            title: None,
            position: None,
            size: Size::new(1024, 768),
            resizable: true,
            maximized: false,
            transparent: false,
            decorations: true,
            always_on_top: false,
        }
    }
}

impl<W: Widget> Debug for WindowBuilder<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowBuilder").finish_non_exhaustive()
    }
}

impl<W: Widget> WindowBuilder<W> {
    /// Creates a new builder with `initializer` used to create the root widget.
    pub fn new<F: FnOnce(&WidgetStorage) -> StyledWidget<W> + 'static>(initializer: F) -> Self {
        Self {
            initializer: Some(Box::new(initializer)),
            configuration: WindowConfiguration::default(),
        }
    }

    /// Sets the window's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.configuration.title = Some(title.into());
        self
    }

    /// Sets the window's position (in screen coordinates).
    pub fn position(mut self, location: Point<i32, Pixels>) -> Self {
        self.configuration.position = Some(location);
        self
    }

    /// Sets the window's size.
    pub fn size(mut self, size: Size<u32, Points>) -> Self {
        self.configuration.size = size;
        self
    }

    /// Prevents the window from being resized.
    pub fn non_resizable(mut self) -> Self {
        self.configuration.resizable = false;
        self
    }

    /// Maximizes the window upon opening.
    pub fn maximize(mut self) -> Self {
        self.configuration.maximized = true;
        self
    }

    /// Enables transparent window handling, if the platform supports it.
    /// Background colors that have transparency will allow other content to
    /// show through.
    pub fn transparent(mut self) -> Self {
        self.configuration.transparent = true;
        self
    }

    /// Removes decorations from the window (such as the title bar).
    pub fn plain(mut self) -> Self {
        self.configuration.decorations = false;
        self
    }

    /// Sets that the window should stay on top of all other windows.
    pub fn always_on_top(mut self) -> Self {
        self.configuration.always_on_top = true;
        self
    }

    /// Opens the window. Only possible on platforms that support multiple windows.
    #[allow(clippy::must_use_candidate)]
    pub fn open(self, frontend: &dyn AnyFrontend) -> bool {
        frontend.open(Box::new(self))
    }
}

/// A [`WindowBuilder`] that has had its widget type parameter erased.
pub trait AnyWindowBuilder: Any {
    /// Casts this value to a mutable [`Any`] reference.
    fn as_mut_any(&mut self) -> &mut dyn Any;
    /// Returns the window configuration.
    fn configuration(&self) -> WindowConfiguration;
    /// Builds the window's root content and returns the registration.
    fn build(&mut self, storage: &WidgetStorage) -> WidgetRegistration;
}

impl<W: Widget> AnyWindowBuilder for WindowBuilder<W> {
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn configuration(&self) -> WindowConfiguration {
        self.configuration.clone()
    }

    fn build(&mut self, storage: &WidgetStorage) -> WidgetRegistration {
        let initializer = self.initializer.take().expect("already built");
        let mut root = initializer(storage);
        // Append the root class to the root widget.
        let mut classes = root.style.get::<Classes>().cloned().unwrap_or_default();
        classes.insert(Cow::from(ROOT_CLASS));
        root.style.push(classes);
        storage.register(root)
    }
}
