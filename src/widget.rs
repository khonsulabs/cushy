use std::clone::Clone;
use std::fmt::Debug;
use std::ops::ControlFlow;
use std::panic::UnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use kludgine::app::winit::error::EventLoopError;
use kludgine::app::winit::event::{DeviceId, KeyEvent, MouseButton};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{Point, Size};

use crate::context::Context;
use crate::dynamic::Dynamic;
use crate::graphics::Graphics;
use crate::window::{RunningWindow, Window, WindowBehavior};
use crate::ConstraintLimit;

pub trait Widget: Send + UnwindSafe + Debug + 'static {
    fn run(self) -> Result<(), EventLoopError>
    where
        Self: Sized,
    {
        Window::<WidgetWindow<Self>>::new(WidgetWindow(Some(self))).run()
    }

    fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context<'_, '_>);

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        graphics: &mut Graphics<'_, '_, '_>,
        context: &mut Context<'_, '_>,
    ) -> Size<UPx>;

    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut Context<'_, '_>) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut Context<'_, '_>) {}

    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut Context<'_, '_>,
    ) -> EventHandling {
        UNHANDLED
    }

    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut Context<'_, '_>,
    ) {
    }

    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut Context<'_, '_>,
    ) {
    }

    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut Context<'_, '_>,
    ) -> EventHandling {
        UNHANDLED
    }
}

pub type EventHandling = ControlFlow<EventHandled, EventIgnored>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EventHandled;
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct EventIgnored;

pub const HANDLED: EventHandling = EventHandling::Break(EventHandled);
pub const UNHANDLED: EventHandling = EventHandling::Continue(EventIgnored);

struct WidgetWindow<W>(Option<W>);

impl<T> WindowBehavior for WidgetWindow<T>
where
    T: Widget + Send + UnwindSafe,
{
    type Context = Self;

    fn initialize(_window: &mut RunningWindow<'_>, context: Self::Context) -> Self {
        context
    }

    fn make_root(&mut self, tree: &crate::tree::Tree) -> crate::tree::ManagedWidget {
        tree.push(self.0.take().expect("root already created"), None)
    }
}

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

impl Eq for BoxedWidget {}

impl PartialEq for BoxedWidget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub enum Value<T>
where
    T: 'static,
{
    Static(T),
    Dynamic(Dynamic<T>),
}

impl<T> Value<T> {
    pub fn map<R>(&mut self, map: impl FnOnce(&T) -> R) -> R {
        match self {
            Value::Static(value) => map(value),
            Value::Dynamic(dynamic) => dynamic.map_ref(map),
        }
    }

    pub fn map_mut<R>(&mut self, map: impl FnOnce(&mut T) -> R) -> R {
        match self {
            Value::Static(value) => map(value),
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
            Value::Static(_) => None,
            Value::Dynamic(value) => Some(value.generation()),
        }
    }
}

pub trait IntoValue<T> {
    fn into_value(self) -> Value<T>;
}

impl<T> IntoValue<T> for T {
    fn into_value(self) -> Value<T> {
        Value::Static(self)
    }
}

impl<'a> IntoValue<String> for &'a str {
    fn into_value(self) -> Value<String> {
        Value::Static(self.to_owned())
    }
}

impl<T> IntoValue<T> for Dynamic<T> {
    fn into_value(self) -> Value<T> {
        Value::Dynamic(self)
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
