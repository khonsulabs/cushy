//! Types for creating reusable widgets (aka components or views).

use std::clone::Clone;
use std::fmt::Debug;
use std::ops::{ControlFlow, Deref};
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Rect, Size};

use crate::context::{EventContext, GraphicsContext};
use crate::styles::Styles;
use crate::tree::{Tree, WidgetId};
use crate::widgets::Style;
use crate::window::{RunningWindow, Window, WindowBehavior};
use crate::{ConstraintLimit, Run};

/// A type that makes up a graphical user interface.
///
/// This type can go by many names in other UI frameworks: View, Component,
/// Control.
pub trait Widget: Send + UnwindSafe + Debug + 'static {
    /// Redraw the contents of this widget.
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>);

    /// Measure this widget and returns the ideal size based on its contents and
    /// the `available_space`.
    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx>;

    /// The widget has been mounted into a parent widget.
    #[allow(unused_variables)]
    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget has been removed from its parent widget.
    #[allow(unused_variables)]
    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {}

    /// Returns true if this widget should respond to mouse input at `location`.
    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

    /// The widget is currently has a cursor hovering it at `location`.
    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer being hovered.
    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget has received focus for user input.
    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer focused for user input.
    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget has become the active widget.
    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut EventContext<'_, '_>) {}

    /// The widget is no longer active.
    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {}

    /// A mouse button event has occurred at `location`. Returns whether the
    /// event has been handled or not.
    ///
    /// If an event is handled, the widget will receive callbacks for
    /// [`mouse_drag`](Self::mouse_drag) and [`mouse_up`](Self::mouse_up).
    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }

    /// A mouse button is being held down as the cursor is moved across the
    /// widget.
    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    /// A mouse button is no longer being pressed.
    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    /// A keyboard event has been sent to this widget. Returns whether the event
    /// has been handled or not.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }

    /// An input manager event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        IGNORED
    }

    /// A mouse wheel event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        IGNORED
    }

    // #[allow(unused_variables)]
    // fn query_component(&self, group: Group, name: &str) -> Option<Component> {
    //     None
    // }

    /// Associates `styles` with this widget.
    ///
    /// This is equivalent to `Style::new(styles, self)`.
    fn with_styles(self, styles: impl Into<Styles>) -> Style
    where
        Self: Sized,
    {
        Style::new(styles, self)
    }
}

impl<T> Run for T
where
    T: Widget,
{
    fn run(self) -> crate::Result {
        BoxedWidget::new(self).run()
    }
}

/// A type that can create a widget.
pub trait MakeWidget: Sized {
    /// Returns a new widget.
    fn make_widget(self) -> BoxedWidget;

    /// Runs the widget this type creates as an application.
    fn run(self) -> crate::Result {
        self.make_widget().run()
    }
}

impl<T> MakeWidget for T
where
    T: Widget,
{
    fn make_widget(self) -> BoxedWidget {
        BoxedWidget::new(self)
    }
}

/// A type that represents whether an event has been handled or ignored.
pub type EventHandling = ControlFlow<EventHandled, EventIgnored>;

/// A marker type that represents a handled event.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

pub struct EventHandled;
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// A marker type that represents an ignored event.
pub struct EventIgnored;

/// An [`EventHandling`] value that represents a handled event.
pub const HANDLED: EventHandling = EventHandling::Break(EventHandled);

/// An [`EventHandling`] value that represents an ignored event.
pub const IGNORED: EventHandling = EventHandling::Continue(EventIgnored);

/// An instance of a [`Widget`].
#[derive(Clone, Debug)]
pub struct BoxedWidget(Arc<Mutex<dyn Widget>>);

impl BoxedWidget {
    /// Returns a new instance containing `widget`.
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self(Arc::new(Mutex::new(widget)))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, dyn Widget> {
        self.0.lock().map_or_else(PoisonError::into_inner, |g| g)
    }
}

impl Run for BoxedWidget {
    fn run(self) -> crate::Result {
        Window::<BoxedWidget>::new(self).run()
    }
}

impl Eq for BoxedWidget {}

impl PartialEq for BoxedWidget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl WindowBehavior for BoxedWidget {
    type Context = Self;

    fn initialize(_window: &mut RunningWindow<'_>, context: Self::Context) -> Self {
        context
    }

    fn make_root(&mut self) -> BoxedWidget {
        self.clone()
    }
}

/// A function that can be invoked with a parameter (`T`) and returns `R`.
///
/// This type is used by widgets to signal various events.
pub struct Callback<T = (), R = ()>(Box<dyn CallbackFunction<T, R>>);

impl<T, R> Debug for Callback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback")
            .field(&(self as *const Self))
            .finish()
    }
}

impl<T, R> Callback<T, R> {
    /// Returns a new instance that calls `function` each time the callback is
    /// invoked.
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(T) -> R + Send + UnwindSafe + 'static,
    {
        Self(Box::new(function))
    }

    /// Invokes the wrapped function and returns the produced value.
    pub fn invoke(&mut self, value: T) -> R {
        self.0.invoke(value)
    }
}

trait CallbackFunction<T, R>: Send + UnwindSafe {
    fn invoke(&mut self, value: T) -> R;
}

impl<T, R, F> CallbackFunction<T, R> for F
where
    F: FnMut(T) -> R + Send + UnwindSafe,
{
    fn invoke(&mut self, value: T) -> R {
        self(value)
    }
}

/// A [`Widget`] that has been attached to a widget hierarchy.
#[derive(Clone)]
pub struct ManagedWidget {
    pub(crate) id: WidgetId,
    pub(crate) widget: BoxedWidget,
    pub(crate) tree: Tree,
}

impl Debug for ManagedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWidget")
            .field("id", &self.id)
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

impl ManagedWidget {
    pub(crate) fn lock(&self) -> MutexGuard<'_, dyn Widget> {
        self.widget.lock()
    }

    pub(crate) fn note_rendered_rect(&self, rect: Rect<Px>) {
        self.tree.note_rendered_rect(self.id, rect);
    }

    /// Returns the region that the widget was last rendered at.
    #[must_use]
    pub fn last_rendered_at(&self) -> Option<Rect<Px>> {
        self.tree.last_rendered_at(self.id)
    }

    /// Returns true if this widget is the currently active widget.
    #[must_use]
    pub fn active(&self) -> bool {
        self.tree.active_widget() == Some(self.id)
    }

    /// Returns true if this widget is currently the hovered widget.
    #[must_use]
    pub fn hovered(&self) -> bool {
        self.tree.hovered_widget() == Some(self.id)
    }

    /// Returns true if this widget is the currently focused widget.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.tree.focused_widget() == Some(self.id)
    }

    /// Returns the parent of this widget.
    #[must_use]
    pub fn parent(&self) -> Option<ManagedWidget> {
        self.tree.parent(self.id).map(|id| self.tree.widget(id))
    }

    pub(crate) fn attach_styles(&self, styles: Styles) {
        self.tree.attach_styles(self.id, styles);
    }
}

impl PartialEq for ManagedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
    }
}

impl PartialEq<BoxedWidget> for ManagedWidget {
    fn eq(&self, other: &BoxedWidget) -> bool {
        &self.widget == other
    }
}

/// A list of [`Widget`]s.
#[derive(Debug, Default)]
#[must_use]
pub struct Widgets {
    ordered: Vec<BoxedWidget>,
}

impl Widgets {
    /// Returns an empty list.
    pub const fn new() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }

    /// Returns a list with enough capacity to hold `capacity` widgets without
    /// reallocation.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            ordered: Vec::with_capacity(capacity),
        }
    }

    /// Pushes `widget` into the list.
    pub fn push<W>(&mut self, widget: W)
    where
        W: MakeWidget,
    {
        self.ordered.push(widget.make_widget());
    }

    /// Adds `widget` to self and returns the updated list.
    pub fn with_widget<W>(mut self, widget: W) -> Self
    where
        W: MakeWidget,
    {
        self.push(widget);
        self
    }

    /// Returns the number of widgets in this list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Returns true if there are no widgets in this list.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }
}

impl<W> FromIterator<W> for Widgets
where
    W: MakeWidget,
{
    fn from_iter<T: IntoIterator<Item = W>>(iter: T) -> Self {
        Self {
            ordered: iter.into_iter().map(MakeWidget::make_widget).collect(),
        }
    }
}

impl Deref for Widgets {
    type Target = [BoxedWidget];

    fn deref(&self) -> &Self::Target {
        &self.ordered
    }
}
