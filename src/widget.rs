//! Types for creating reusable widgets (aka components or views).

use std::any::Any;
use std::clone::Clone;
use std::fmt::Debug;
use std::ops::{ControlFlow, Deref, DerefMut};
use std::panic::UnwindSafe;
use std::sync::atomic::{self, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, IntoUnsigned, Point, Rect, Size};

use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::VisualOrder;
use crate::styles::{IntoComponentValue, NamedComponent, Styles, ThemePair};
use crate::tree::Tree;
use crate::value::{IntoValue, Value};
use crate::widgets::{Align, Expand, Scroll, Style};
use crate::window::{RunningWindow, ThemeMode, Window, WindowBehavior};
use crate::{ConstraintLimit, Run};

/// A type that makes up a graphical user interface.
///
/// This type can go by many names in other UI frameworks: View, Component,
/// Control.
pub trait Widget: Send + UnwindSafe + Debug + 'static {
    /// Redraw the contents of this widget.
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>);

    /// Layout this widget and returns the ideal size based on its contents and
    /// the `available_space`.
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
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

    /// This widget has been targeted to be focused. If this function returns
    /// true, the widget will be focused. If false, Gooey will continue
    /// searching for another focus target.
    #[allow(unused_variables)]
    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

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

    /// Returns a reference to a single child widget if this widget is a widget
    /// that primarily wraps a single other widget to customize its behavior.
    #[must_use]
    fn wraps(&mut self) -> Option<&WidgetInstance> {
        None
    }
}

impl<T> Run for T
where
    T: MakeWidget,
{
    fn run(self) -> crate::Result {
        self.make_widget().run()
    }
}

/// A [`Widget`] that contains a single child.
pub trait WrapperWidget: Debug + Send + UnwindSafe + 'static {
    /// Returns the child widget.
    fn child_mut(&mut self) -> &mut WidgetRef;

    /// Returns the rectangle that the child widget should occupy given
    /// `available_space`.
    #[allow(unused_variables)]
    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Rect<Px> {
        let child = self.child_mut().mounted(&mut context.as_event_context());

        context
            .for_other(&child)
            .layout(available_space)
            .into_signed()
            .into()
    }

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

    /// This widget has been targeted to be focused. If this function returns
    /// true, the widget will be focused. If false, Gooey will continue
    /// searching for another focus target.
    #[allow(unused_variables)]
    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

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
}

impl<T> Widget for T
where
    T: WrapperWidget,
{
    fn wraps(&mut self) -> Option<&WidgetInstance> {
        Some(self.child_mut().widget())
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let child = self.child_mut().mounted(&mut context.as_event_context());
        context.for_other(&child).redraw();
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let child = self.child_mut().mounted(&mut context.as_event_context());

        let layout = self.layout_child(available_space, context);
        context.set_child_layout(&child, layout);
        layout.size.into_unsigned()
    }

    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {
        T::mounted(self, context);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {
        T::unmounted(self, context);
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) -> bool {
        T::hit_test(self, location, context)
    }

    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) {
        T::hover(self, location, context);
    }

    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {
        T::unhover(self, context);
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        T::accept_focus(self, context)
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        T::focus(self, context);
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        T::blur(self, context);
    }

    fn activate(&mut self, context: &mut EventContext<'_, '_>) {
        T::activate(self, context);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {
        T::deactivate(self, context);
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        T::mouse_down(self, location, device_id, button, context)
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
        T::mouse_drag(self, location, device_id, button, context);
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
        T::mouse_up(self, location, device_id, button, context);
    }

    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        T::keyboard_input(self, device_id, input, is_synthetic, context)
    }

    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        T::ime(self, ime, context)
    }

    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        T::mouse_wheel(self, device_id, delta, phase, context)
    }
}

/// A type that can create a [`WidgetInstance`].
pub trait MakeWidget: Sized {
    /// Returns a new widget.
    fn make_widget(self) -> WidgetInstance;

    /// Returns a new window containing `self` as the root widget.
    fn into_window(self) -> Window<WidgetInstance> {
        Window::new(self.make_widget())
    }

    /// Associates `styles` with this widget.
    ///
    /// This is equivalent to `Style::new(styles, self)`.
    fn with_styles(self, styles: impl IntoValue<Styles>) -> Style
    where
        Self: Sized,
    {
        Style::new(styles, self)
    }

    /// Associates a style component with `self`.
    fn with(self, name: &impl NamedComponent, component: impl IntoComponentValue) -> Style {
        let mut styles = Styles::new();
        styles.insert(name, component);
        Style::new(styles, self)
    }

    /// Sets the widget that should be focused next.
    ///
    /// Gooey automatically determines reverse tab order by using this same
    /// relationship.
    fn with_next_focus(self, next_focus: impl IntoValue<Option<WidgetId>>) -> WidgetInstance {
        self.make_widget().with_next_focus(next_focus)
    }

    /// Sets this widget as a "default" widget.
    ///
    /// Default widgets are automatically activated when the user signals they
    /// are ready for the default action to occur.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Submit buttons on forms
    /// - Ok buttons
    #[must_use]
    fn into_default(self) -> WidgetInstance {
        self.make_widget().into_default()
    }

    /// Sets this widget as an "escape" widget.
    ///
    /// Escape widgets are automatically activated when the user signals they
    /// are ready to escape their current situation.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Close buttons
    /// - Cancel buttons
    #[must_use]
    fn into_escape(self) -> WidgetInstance {
        self.make_widget().into_escape()
    }

    /// Returns a collection of widgets using `self` and `other`.
    fn and(self, other: impl MakeWidget) -> Children {
        let mut children = Children::new();
        children.push(self);
        children.push(other);
        children
    }

    /// Expands `self` to grow to fill its parent.
    #[must_use]
    fn expand(self) -> Expand {
        Expand::new(self)
    }

    /// Expands `self` to grow to fill its parent proportionally with other
    /// weighted siblings.
    #[must_use]
    fn expand_weighted(self, weight: u8) -> Expand {
        Expand::weighted(weight, self)
    }

    /// Aligns `self` to the center vertically and horizontally.
    #[must_use]
    fn centered(self) -> Align {
        Align::centered(self)
    }

    /// Aligns `self` to the left.
    fn align_left(self) -> Align {
        self.centered().align_left()
    }

    /// Aligns `self` to the right.
    fn align_right(self) -> Align {
        self.centered().align_right()
    }

    /// Aligns `self` to the top.
    fn align_top(self) -> Align {
        self.centered().align_top()
    }

    /// Aligns `self` to the bottom.
    fn align_bottom(self) -> Align {
        self.centered().align_bottom()
    }

    /// Fits `self` horizontally within its parent.
    fn fit_horizontally(self) -> Align {
        self.centered().fit_horizontally()
    }

    /// Fits `self` vertically within its parent.
    fn fit_vertically(self) -> Align {
        self.centered().fit_vertically()
    }

    /// Allows scrolling `self` both vertically and horizontally.
    #[must_use]
    fn scroll(self) -> Scroll {
        Scroll::new(self)
    }

    /// Allows scrolling `self` vertically.
    #[must_use]
    fn vertical_scroll(self) -> Scroll {
        Scroll::vertical(self)
    }

    /// Allows scrolling `self` horizontally.
    #[must_use]
    fn horizontal_scroll(self) -> Scroll {
        Scroll::horizontal(self)
    }

    /// Creates a [`WidgetRef`] for use as child widget.
    #[must_use]
    fn widget_ref(self) -> WidgetRef {
        WidgetRef::new(self)
    }
}

/// A type that can create a [`WidgetInstance`] with a preallocated
/// [`WidgetId`].
pub trait MakeWidgetWithId: Sized {
    /// Returns a new [`WidgetInstance`] whose [`WidgetId`] is `id`.
    fn make_with_id(self, id: WidgetTag) -> WidgetInstance;
}

impl<T> MakeWidgetWithId for T
where
    T: Widget,
{
    fn make_with_id(self, id: WidgetTag) -> WidgetInstance {
        WidgetInstance::with_id(self, id)
    }
}

impl<T> MakeWidget for T
where
    T: MakeWidgetWithId,
{
    fn make_widget(self) -> WidgetInstance {
        self.make_with_id(WidgetTag::unique())
    }
}

impl MakeWidget for WidgetInstance {
    fn make_widget(self) -> WidgetInstance {
        self
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

pub(crate) trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An instance of a [`Widget`].
#[derive(Clone, Debug)]
pub struct WidgetInstance {
    data: Arc<WidgetInstanceData>,
}

#[derive(Debug)]
struct WidgetInstanceData {
    id: WidgetId,
    default: bool,
    cancel: bool,
    next_focus: Value<Option<WidgetId>>,
    widget: Box<Mutex<dyn AnyWidget>>,
}

impl WidgetInstance {
    /// Returns a new instance containing `widget` that is assigned the unique
    /// `id` provided.
    pub fn with_id<W>(widget: W, id: WidgetTag) -> Self
    where
        W: Widget,
    {
        Self {
            data: Arc::new(WidgetInstanceData {
                id: id.into(),
                next_focus: Value::default(),
                default: false,
                cancel: false,
                widget: Box::new(Mutex::new(widget)),
            }),
        }
    }

    /// Returns a new instance containing `widget`.
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::with_id(widget, WidgetTag::unique())
    }

    /// Returns the unique id of this widget instance.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.data.id
    }

    /// Sets the widget that should be focused next.
    ///
    /// Gooey automatically determines reverse tab order by using this same
    /// relationship.
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn with_next_focus(
        mut self,
        next_focus: impl IntoValue<Option<WidgetId>>,
    ) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.next_focus = next_focus.into_value();
        self
    }

    /// Sets this widget as a "default" widget.
    ///
    /// Default widgets are automatically activated when the user signals they
    /// are ready for the default action to occur.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Submit buttons on forms
    /// - Ok buttons
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn into_default(mut self) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.default = true;
        self
    }

    /// Sets this widget as an "escape" widget.
    ///
    /// Escape widgets are automatically activated when the user signals they
    /// are ready to escape their current situation.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Close buttons
    /// - Cancel buttons
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn into_escape(mut self) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.cancel = true;
        self
    }

    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    pub fn lock(&self) -> WidgetGuard<'_> {
        WidgetGuard(
            self.data
                .widget
                .lock()
                .map_or_else(PoisonError::into_inner, |g| g),
        )
    }

    /// Runs this widget instance as an application.
    pub fn run(self) -> crate::Result {
        Window::<WidgetInstance>::new(self).run()
    }

    /// Returns the id of the widget that should receive focus after this
    /// widget.
    ///
    /// This value comes from [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn next_focus(&self) -> Option<WidgetId> {
        self.data.next_focus.get()
    }

    /// Returns true if this is a default widget.
    ///
    /// See [`MakeWidget::into_default()`] for more information.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.data.default
    }

    /// Returns true if this is an escape widget.
    ///
    /// See [`MakeWidget::into_escape()`] for more information.
    #[must_use]
    pub fn is_escape(&self) -> bool {
        self.data.cancel
    }
}

impl AsRef<WidgetId> for WidgetInstance {
    fn as_ref(&self) -> &WidgetId {
        &self.data.id
    }
}

impl Eq for WidgetInstance {}

impl PartialEq for WidgetInstance {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl WindowBehavior for WidgetInstance {
    type Context = Self;

    fn initialize(_window: &mut RunningWindow<'_>, context: Self::Context) -> Self {
        context
    }

    fn make_root(&mut self) -> WidgetInstance {
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
    pub(crate) widget: WidgetInstance,
    pub(crate) tree: Tree,
}

impl Debug for ManagedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWidget")
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

impl ManagedWidget {
    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    #[must_use]
    pub fn lock(&self) -> WidgetGuard<'_> {
        self.widget.lock()
    }

    pub(crate) fn set_layout(&self, rect: Rect<Px>) {
        self.tree.set_layout(self.id(), rect);
    }

    /// Returns the unique id of this widget instance.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.widget.id()
    }

    /// Returns the next widget to focus after this widget.
    ///
    /// This function returns the value set in
    /// [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn next_focus(&self) -> Option<ManagedWidget> {
        self.widget
            .next_focus()
            .and_then(|next_focus| self.tree.widget(next_focus))
    }

    /// Returns the region that the widget was last rendered at.
    #[must_use]
    pub fn last_layout(&self) -> Option<Rect<Px>> {
        self.tree.layout(self.id())
    }

    /// Returns true if this widget is the currently active widget.
    #[must_use]
    pub fn active(&self) -> bool {
        self.tree.active_widget() == Some(self.id())
    }

    /// Returns true if this widget is currently the hovered widget.
    #[must_use]
    pub fn hovered(&self) -> bool {
        self.tree.is_hovered(self.id())
    }

    /// Returns true if this widget that is directly beneath the cursor.
    #[must_use]
    pub fn primary_hover(&self) -> bool {
        self.tree.hovered_widget() == Some(self.id())
    }

    /// Returns true if this widget is the currently focused widget.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.tree.focused_widget() == Some(self.id())
    }

    /// Returns the parent of this widget.
    #[must_use]
    pub fn parent(&self) -> Option<ManagedWidget> {
        self.tree
            .parent(self.id())
            .and_then(|id| self.tree.widget(id))
    }

    /// Returns true if this node has a parent.
    #[must_use]
    pub fn has_parent(&self) -> bool {
        self.tree.parent(self.id()).is_some()
    }

    pub(crate) fn attach_styles(&self, styles: Value<Styles>) {
        self.tree.attach_styles(self.id(), styles);
    }

    pub(crate) fn attach_theme(&self, theme: Value<ThemePair>) {
        self.tree.attach_theme(self.id(), theme);
    }

    pub(crate) fn attach_theme_mode(&self, theme: Value<ThemeMode>) {
        self.tree.attach_theme_mode(self.id(), theme);
    }

    pub(crate) fn overidden_theme(&self) -> (Option<Value<ThemePair>>, Option<Value<ThemeMode>>) {
        self.tree.overriden_theme(self.id())
    }

    pub(crate) fn reset_child_layouts(&self) {
        self.tree.reset_child_layouts(self.id());
    }

    pub(crate) fn visually_ordered_children(&self, order: VisualOrder) -> Vec<ManagedWidget> {
        self.tree.visually_ordered_children(self.id(), order)
    }
}

impl AsRef<WidgetId> for ManagedWidget {
    fn as_ref(&self) -> &WidgetId {
        self.widget.as_ref()
    }
}

impl PartialEq for ManagedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
    }
}

impl PartialEq<WidgetInstance> for ManagedWidget {
    fn eq(&self, other: &WidgetInstance) -> bool {
        &self.widget == other
    }
}

/// Exclusive access to a widget.
///
/// This type is powered by a `Mutex`, which means care must be taken to prevent
/// deadlocks.
pub struct WidgetGuard<'a>(MutexGuard<'a, dyn AnyWidget>);

impl WidgetGuard<'_> {
    pub(crate) fn as_widget(&mut self) -> &mut dyn AnyWidget {
        &mut *self.0
    }

    /// Returns a reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0.as_any().downcast_ref()
    }

    /// Returns an exclusive reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0.as_any_mut().downcast_mut()
    }
}

/// A list of [`Widget`]s.
#[derive(Debug, Default)]
#[must_use]
pub struct Children {
    ordered: Vec<WidgetInstance>,
}

impl Children {
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

    /// Inserts `widget` into the list at `index`.
    pub fn insert<W>(&mut self, index: usize, widget: W)
    where
        W: MakeWidget,
    {
        self.ordered.insert(index, widget.make_widget());
    }

    /// Adds `widget` to self and returns the updated list.
    pub fn and<W>(mut self, widget: W) -> Self
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

    /// Truncates the collection of children to `length`.
    ///
    /// If this collection is already smaller or the same size as `length`, this
    /// function does nothing.
    pub fn truncate(&mut self, length: usize) {
        self.ordered.truncate(length);
    }
}

impl<W> FromIterator<W> for Children
where
    W: MakeWidget,
{
    fn from_iter<T: IntoIterator<Item = W>>(iter: T) -> Self {
        Self {
            ordered: iter.into_iter().map(MakeWidget::make_widget).collect(),
        }
    }
}

impl Deref for Children {
    type Target = [WidgetInstance];

    fn deref(&self) -> &Self::Target {
        &self.ordered
    }
}

impl DerefMut for Children {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ordered
    }
}

/// A child widget
#[derive(Debug, Clone)]
pub enum WidgetRef {
    /// An unmounted child widget
    Unmounted(WidgetInstance),
    /// A mounted child widget
    Mounted(ManagedWidget),
}

impl WidgetRef {
    /// Returns a new unmounted child
    pub fn new(widget: impl MakeWidget) -> Self {
        Self::Unmounted(widget.make_widget())
    }

    /// Returns this child, mounting it in the process if necessary.
    pub fn mounted(&mut self, context: &mut EventContext<'_, '_>) -> ManagedWidget {
        if let WidgetRef::Unmounted(instance) = self {
            *self = WidgetRef::Mounted(context.push_child(instance.clone()));
        }

        let Self::Mounted(widget) = self else {
            unreachable!("just initialized")
        };
        widget.clone()
    }

    /// Returns the a reference to the underlying widget instance.
    #[must_use]
    pub fn widget(&self) -> &WidgetInstance {
        match self {
            WidgetRef::Unmounted(widget) => widget,
            WidgetRef::Mounted(managed) => &managed.widget,
        }
    }
}

impl AsRef<WidgetId> for WidgetRef {
    fn as_ref(&self) -> &WidgetId {
        match self {
            WidgetRef::Unmounted(widget) => widget.as_ref(),
            WidgetRef::Mounted(widget) => widget.as_ref(),
        }
    }
}

/// The unique id of a [`WidgetInstance`].
///
/// Each [`WidgetInstance`] is guaranteed to have a unique [`WidgetId`] across
/// the lifetime of an application.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    fn unique() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, atomic::Ordering::Acquire))
    }
}

/// A [`WidgetId`] that has not been assigned to a [`WidgetInstance`].
///
/// This type is passed to [`MakeWidgetWithId::make_with_id()`] to create a
/// [`WidgetInstance`] with a preallocated id.
///
/// This type cannot be cloned or copied to ensure only a single widget can be
/// assigned a given [`WidgetId`]. The contained [`WidgetId`] can be accessed
/// via [`id()`](Self::id), `Into<WidgetId>`, or `Deref`.
#[derive(Eq, PartialEq, Debug)]
pub struct WidgetTag(WidgetId);

impl WidgetTag {
    /// Returns a unique tag and its contained id.
    #[must_use]
    pub fn new() -> (Self, WidgetId) {
        let tag = Self::unique();
        let id = *tag;
        (tag, id)
    }

    /// Returns a newly allocated [`WidgetId`] that is guaranteed to be unique
    /// for the lifetime of the application.
    #[must_use]
    pub fn unique() -> Self {
        Self(WidgetId::unique())
    }

    /// Returns the contained widget id.
    #[must_use]
    pub const fn id(&self) -> WidgetId {
        self.0
    }
}

impl From<WidgetTag> for WidgetId {
    fn from(value: WidgetTag) -> Self {
        value.0
    }
}

impl Deref for WidgetTag {
    type Target = WidgetId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
