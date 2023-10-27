use std::clone::Clone;
use std::fmt::Debug;
use std::ops::ControlFlow;
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kludgine::app::winit::error::EventLoopError;
use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Rect, Size};

use crate::context::{EventContext, GraphicsContext};
use crate::dynamic::Dynamic;
use crate::styles::{Component, Group, Styles};
use crate::tree::{Tree, WidgetId};
use crate::widgets::Style;
use crate::window::{RunningWindow, Window, WindowBehavior};
use crate::{ConstraintLimit, Run};

pub trait Widget: Send + UnwindSafe + Debug + 'static {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>);

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx>;

    #[allow(unused_variables)]
    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {}
    #[allow(unused_variables)]
    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut EventContext<'_, '_>) {}

    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        UNHANDLED
    }

    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
    }

    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        UNHANDLED
    }
    #[allow(unused_variables)]
    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        UNHANDLED
    }

    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        UNHANDLED
    }

    #[allow(unused_variables)]
    fn query_component(&self, group: Group, name: &str) -> Option<Component> {
        None
    }

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
    fn run(self) -> crate::Result<(), EventLoopError> {
        BoxedWidget::new(self).run()
    }
}

pub trait MakeWidget: Sized {
    fn make_widget(self) -> BoxedWidget;

    fn run(self) -> Result<(), EventLoopError> {
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

pub type EventHandling = ControlFlow<EventHandled, EventIgnored>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EventHandled;
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EventIgnored;

pub const HANDLED: EventHandling = EventHandling::Break(EventHandled);
pub const UNHANDLED: EventHandling = EventHandling::Continue(EventIgnored);

#[derive(Clone, Debug)]
pub struct BoxedWidget(Arc<Mutex<dyn Widget>>);

impl BoxedWidget {
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
    fn run(self) -> crate::Result<(), EventLoopError> {
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

#[derive(Debug)]
pub enum Value<T> {
    Constant(T),
    Dynamic(Dynamic<T>),
}

impl<T> Value<T> {
    pub fn dynamic(value: T) -> Self {
        Self::Dynamic(Dynamic::new(value))
    }

    pub fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    pub fn map<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_ref(map),
        }
    }

    pub fn map_mut<R>(&mut self, map: impl FnOnce(&mut T) -> R) -> R {
        match self {
            Value::Constant(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_mut(map),
        }
    }

    pub fn get(&mut self) -> T
    where
        T: Clone,
    {
        self.map(Clone::clone)
    }

    pub fn generation(&self) -> Option<usize> {
        match self {
            Value::Constant(_) => None,
            Value::Dynamic(value) => Some(value.generation()),
        }
    }
}

pub trait IntoValue<T> {
    fn into_value(self) -> Value<T>;
}

impl<T> IntoValue<T> for T {
    fn into_value(self) -> Value<T> {
        Value::Constant(self)
    }
}

impl<'a> IntoValue<String> for &'a str {
    fn into_value(self) -> Value<String> {
        Value::Constant(self.to_owned())
    }
}

impl<T> IntoValue<T> for Dynamic<T> {
    fn into_value(self) -> Value<T> {
        Value::Dynamic(self)
    }
}

impl<T> IntoValue<T> for Value<T> {
    fn into_value(self) -> Value<T> {
        self
    }
}

pub struct Callback<T>(Box<dyn CallbackFunction<T>>);

impl<T> Debug for Callback<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback")
            .field(&(self as *const Self))
            .finish()
    }
}

impl<T> Callback<T> {
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(T) + Send + UnwindSafe + 'static,
    {
        Self(Box::new(function))
    }

    pub fn invoke(&mut self, value: T) {
        self.0.invoke(value);
    }
}

trait CallbackFunction<T>: Send + UnwindSafe {
    fn invoke(&mut self, value: T);
}
impl<T, F> CallbackFunction<T> for F
where
    F: FnMut(T) + Send + UnwindSafe,
{
    fn invoke(&mut self, value: T) {
        self(value);
    }
}

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

    #[must_use]
    pub fn last_rendered_at(&self) -> Option<Rect<Px>> {
        self.tree.last_rendered_at(self.id)
    }

    #[must_use]
    pub fn active(&self) -> bool {
        self.tree.active_widget() == Some(self.id)
    }

    #[must_use]
    pub fn hovered(&self) -> bool {
        self.tree.hovered_widget() == Some(self.id)
    }

    #[must_use]
    pub fn focused(&self) -> bool {
        self.tree.focused_widget() == Some(self.id)
    }

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
